use std::cmp::*;
use std::collections::hash_map::RandomState;
use std::collections::HashSet;

use chrono::prelude::*;
use std::error::Error;
use trees::{tr, Forest, ForestWalk, Visit};
use yatt_orm::errors::{DBError, DBResult};
use yatt_orm::sqlite::DB;
use yatt_orm::statement::*;
use yatt_orm::{FieldVal, Identifiers, Storage};

type PinNode<'a> = std::pin::Pin<&'a mut trees::Node<Node>>;

#[derive(Debug)]
pub(crate) struct ImportedPath {
  // nodes: Vec<Node>,
// source: String,
// comment: String,
}

pub(crate) trait PathSource {
  fn get_path(
    &self,
    source: &str,
  ) -> Result<ImportedPath, Box<dyn Error>>;
}

pub trait DBRoot: Storage {
  fn cur_running(&self) -> DBResult<Option<(Node, Interval)>>
  where
    Self: Sized,
  {
    let mut interval: Vec<Interval> =
      self.get_by_filter(eq(Interval::end_n(), FieldVal::Null))?;

    if interval.is_empty() {
      return Ok(None);
    }

    let interval = interval.pop().unwrap();

    let node: Vec<Node> = self
      .get_by_filter(eq(Node::id_n(), interval.node_id.unwrap()))?;

    if node.is_empty() {
      return Err(DBError::Unexpected {
        message: format!(
          "Task with id={} for interval with id={}, not exists",
          interval.node_id.unwrap_or(0),
          interval.id,
        ),
      });
    }

    let node = node[0].clone();
    Ok(Some((node, interval)))
  }

  fn last_running(&self) -> DBResult<Option<(Node, Interval)>>
  where
    Self: Sized,
  {
    let interval: Vec<Interval> = self.get_by_statement(
      filter(ne(Interval::deleted_n(), 1))
        .sort(Interval::end_n(), SortDir::Descend)
        .limit(1),
    )?;

    if interval.is_empty() {
      return Ok(None);
    }

    let interval = interval.first().unwrap();

    let node: Vec<Node> = self
      .get_by_filter(eq(Node::id_n(), interval.node_id.unwrap()))?;

    if node.is_empty() {
      return Err(DBError::Unexpected {
        message: format!(
          "Task with id={} for interval with id={}, not exists",
          interval.node_id.unwrap_or(0),
          interval.id,
        ),
      });
    }

    let node = node[0].clone();
    Ok(Some((node, interval.to_owned())))
  }

  fn find_path(&self, path: &[&str]) -> DBResult<Vec<Node>>
  where
    Self: Sized,
  {
    let mut parent = FieldVal::Null;
    let mut res = Vec::new();
    for p in path.iter() {
      let node = self.find_path_part(p, &parent)?;
      if let Some(node) = node {
        parent = FieldVal::Usize(node.id);
        res.push(node);
      } else {
        return Ok(res);
      }
    }
    Ok(res)
  }

  /// Crates all not exist node of path, and returns all nodes.
  fn create_path(&self, path: &[&str]) -> DBResult<Vec<Node>>
  where
    Self: Sized,
  {
    if path.is_empty() {
      return Err(DBError::Unexpected {
        message: "provided value for path is empty".to_string(),
      });
    }

    let mut nodes = self.find_path(path)?;
    let p_len = path.len();
    let n_len = nodes.len();
    let high = p_len - (p_len - n_len);

    for node in nodes.iter_mut().filter(|n| n.deleted) {
      node.deleted = false;
      self.save(node)?;
    }

    if high == p_len {
      return Ok(nodes);
    }

    let mut parent_id = None;
    if !nodes.is_empty() {
      parent_id = Some(nodes.last().unwrap().id)
    }
    for n in path.iter().take(p_len).skip(high) {
      let mut node = Node {
        parent_id,
        label: n.to_string(),
        ..Node::default()
      };

      let id = self.save(&node)?;
      node.id = id;
      parent_id = Some(id);
      nodes.push(node);
    }

    Ok(nodes)
  }

  /// Returns ancestors of node with given id, including
  /// the node with given id itself.
  fn ancestors(&self, id: usize) -> DBResult<Vec<Node>>
  where
    Self: Sized,
  {
    let mut res = Vec::new();
    let mut next = Some(id);

    while next.is_some() {
      let node: Node = self.get_by_id(next.unwrap())?;
      next = node.parent_id;
      res.push(node);
    }

    res.reverse();

    Ok(res)
  }

  fn find_path_part(
    &self,
    name: &str,
    parent_id: &FieldVal,
  ) -> DBResult<Option<Node>>
  where
    Self: Sized,
  {
    let mut nodes: Vec<Node> = self.get_by_filter(and(
      eq(Node::parent_id_n(), parent_id),
      eq(Node::label_n(), name),
    ))?;
    if nodes.is_empty() {
      return Ok(None);
    };

    if nodes.len() > 1 {
      return Err(DBError::Unexpected {
        message: "query return multiple rows".to_string(),
      });
    };
    Ok(Some(nodes.remove(0)))
  }

  fn get_list_with_ancestors(
    &self,
    filt: Filter,
  ) -> DBResult<Vec<Node>>
  where
    Self: Sized,
  {
    let stmt = filter(and(
      filt,
      not(exists(from("nodes").filter(and(
        eq(Node::parent_id_n(), FieldVal::FieldName("id".into())),
        eq(Node::deleted_n(), 0),
      )))),
    ))
    .recursive_on(Node::parent_id_n())
    .sort(Node::parent_id_n(), SortDir::Ascend)
    .sort(Node::id_n(), SortDir::Ascend);

    let matched: Vec<Node> = self.get_by_statement(stmt)?;
    Ok(matched)
  }

  fn get_filtered_forest(
    &self,
    filt: Filter,
  ) -> DBResult<Option<Forest<Node>>>
  where
    Self: Sized,
  {
    let mut matched: Vec<Node> =
      self.get_list_with_ancestors(filt)?;
    if matched.is_empty() {
      return Ok(None);
    }

    let mut root: Forest<Node> = Forest::new();

    let first_child = matched
      .iter()
      .enumerate()
      .find(|(_, el)| el.parent_id.is_some())
      .map(|(idx, _)| idx)
      .unwrap_or(matched.len());

    if first_child > 0 {
      for node in matched.drain(0..first_child) {
        root.push_back(tr(node));
      }
      if matched.is_empty() {
        return Ok(Some(root));
      }
    }

    let mut dangling: Forest<Node> = Forest::new();

    for node in matched.drain(..) {
      adopt_node(node, &mut root, &mut dangling);
    }

    root.append(dangling);

    Ok(Some(root))
  }

  /// Returns deleted intervals count
  fn remove_intervals(&self, node_id: usize) -> DBResult<usize>
  where
    Self: Sized,
  {
    self.remove_by_filter::<Interval>(and(
      eq(Interval::node_id_n(), node_id),
      ne(Interval::deleted_n(), 1),
    ))
  }

  // Return deleted nodes and intervals count
  fn remove_children(
    &self,
    node_id: usize,
  ) -> DBResult<(usize, usize)>
  where
    Self: Sized,
  {
    let children: Vec<Node> = self.get_by_filter(and(
      eq(Node::parent_id_n(), node_id),
      ne(Node::deleted_n(), 1),
    ))?;

    let mut node_cnt = 0;
    let mut interval_cnt = 0;
    for node in children {
      let (n, i) = self.remove_children(node.id)?;
      node_cnt += n;
      interval_cnt += i;
      interval_cnt += self.remove_intervals(node.id)?;
    }

    node_cnt += self.remove_by_filter::<Node>(and(
      eq(Node::parent_id_n(), node_id),
      ne(Node::deleted_n(), 1),
    ))?;

    Ok((node_cnt, interval_cnt))
  }

  /// Removes node, all it's children with intervals
  /// returns removed nodes and intervals count
  fn remove_node(&self, node_id: usize) -> DBResult<(usize, usize)>
  where
    Self: Sized,
  {
    let mut node_cnt = 0;
    let mut interval_cnt = 0;

    let (n, i) = self.remove_children(node_id)?;
    node_cnt += n;
    interval_cnt += i;

    interval_cnt += self.remove_intervals(node_id)?;
    node_cnt += self.remove_by_filter::<Node>(eq("id", node_id))?;

    Ok((node_cnt, interval_cnt))
  }

  fn has_children(&self, node_id: usize) -> DBResult<bool>
  where
    Self: Sized,
  {
    Ok(
      self
        .get_by_statement::<Node>(
          filter(and(
            eq(Node::parent_id_n(), node_id),
            ne(Node::deleted_n(), 1),
          ))
          .limit(1),
        )?
        .len()
        == 1,
    )
  }
}

fn adopt_node(
  node: Node,
  root: &mut Forest<Node>,
  dangling: &mut Forest<Node>,
) {
  if let Some(node) = try_adopt_node(node, dangling) {
    if let Some(node) = try_adopt_node(node, root) {
      dangling.push_back(tr(node));
    }
  }
}

fn try_adopt_node(
  node: Node,
  forest: &mut Forest<Node>,
) -> Option<Node> {
  let mut node = node;

  for tree in forest.iter_mut() {
    node = try_add_child(tree, node)?
  }

  Some(node)
}
fn try_add_child(tree: PinNode, node: Node) -> Option<Node> {
  let mut tree = tree;
  if tree.data.id == node.parent_id.unwrap() {
    tree.push_back(tr(node));
    None
  } else {
    let mut node = node;
    for child in tree.iter_mut() {
      let res = try_add_child(child, node);

      if let Some(res) = res {
        node = res;
        continue;
      }

      return None;
    }
    Some(node)
  }
}

pub struct FlattenForestIter<'a, T: Clone> {
  walk: &'a mut ForestWalk<T>,
  path: Vec<T>,
}

impl<'a, T: Clone> FlattenForestIter<'a, T> {
  pub fn new(walk: &'a mut ForestWalk<T>) -> Self {
    FlattenForestIter { walk, path: vec![] }
  }
}

impl<'a, T: Clone> Iterator for FlattenForestIter<'a, T> {
  type Item = Vec<T>;
  fn next(&mut self) -> Option<Self::Item> {
    while let Some(next) = self.walk.get() {
      let need_return = match next {
        Visit::Begin(node) => {
          self.path.push(node.data.clone());
          false
        }
        Visit::Leaf(node) => {
          self.path.push(node.data.clone());
          true
        }
        Visit::End(_) => {
          self.path.pop();
          false
        }
      };
      self.walk.forward();
      if need_return {
        let res = self.path.clone();
        self.path.pop();
        return Some(res);
      }
    }
    None
  }
}
#[allow(clippy::derive_ord_xor_partial_ord)]
#[derive(Debug, Clone, Identifiers, PartialEq, Eq, Ord)]
pub struct Node {
  pub id: usize,
  pub parent_id: Option<usize>,
  pub label: String,
  pub created: DateTime<Utc>,
  pub closed: bool,
  pub deleted: bool,
  pub tags: String,
}

impl PartialOrd for Node {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.label.cmp(&other.label))
  }
}

impl Default for Node {
  fn default() -> Self {
    Node {
      id: 0,
      parent_id: None,
      label: String::new(),
      created: Utc::now(),
      closed: false,
      deleted: false,
      tags: String::new(),
    }
  }
}
impl ToString for Node {
  fn to_string(&self) -> String {
    self.label.to_owned()
  }
}
impl Node {
  pub fn get_tags(&self) -> Vec<String> {
    self
      .tags
      .trim_matches(',')
      .split(',')
      .map(String::from)
      .collect()
  }
  pub fn set_tags(&mut self, tags: &[String]) {
    let tags: HashSet<String, RandomState> =
      tags.iter().map(String::from).collect();
    self.tags = format!(
      ",{},",
      tags.iter().map(String::from).collect::<Vec<_>>().join(","),
    );
  }
  pub fn add_tags(&mut self, tags: &[String]) {
    let mut new_tags = self.get_tags();
    for tag in tags {
      new_tags.push(tag.to_owned());
    }
    self.set_tags(&new_tags);
  }
  pub fn remove_tags(&mut self, tags: &[String]) {
    let cur_tags = self.get_tags();
    let mut new_tags = Vec::new();

    for tag in cur_tags {
      if !tags.contains(&tag) {
        new_tags.push(tag)
      }
    }
    self.set_tags(&new_tags);
  }
  pub fn get_comma_tags(&self) -> Vec<String> {
    self.get_tags().iter().map(|v| format!(",{},", v)).collect()
  }
}

#[derive(Debug, Clone, Copy, Identifiers)]
pub struct Interval {
  pub id: usize,
  pub node_id: Option<usize>,
  pub begin: DateTime<Utc>,
  pub end: Option<DateTime<Utc>>,
  pub deleted: bool,
  pub closed: bool,
}

impl Default for Interval {
  fn default() -> Self {
    Interval {
      id: 0,
      node_id: None,
      begin: Utc::now(),
      end: None,
      deleted: false,
      closed: false,
    }
  }
}
impl ToString for Interval {
  fn to_string(&self) -> String {
    let end = match self.end {
      Some(d) => d.to_rfc3339(),
      None => "never".to_string(),
    };
    format!("[started: {} stopped: {}]", self.begin, end)
  }
}

impl DBRoot for DB<'_> {}
