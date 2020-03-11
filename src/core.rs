use std::convert::TryFrom;

use chrono::prelude::*;
use trees::{tr, Forest, ForestWalk, Visit};
use yatt_orm::errors::{DBError, DBResult};
use yatt_orm::statement::*;
use yatt_orm::FieldVal;
use yatt_orm::{Identifiers, Storage};

type PinNode<'a> = std::pin::Pin<&'a mut trees::Node<Node>>;

pub trait DBRoot: Storage {
    fn cur_running(&self) -> DBResult<Option<(Node, Interval)>>
    where
        Self: Sized,
    {
        let mut interval: Vec<Interval> = self
            .get_by_filter(eq(Interval::end_n(), FieldVal::Null))?;

        if interval.is_empty() {
            return Ok(None);
        }

        let interval = interval.pop().unwrap();

        let node: Vec<Node> = self.get_by_filter(eq(
            Node::id_n(),
            interval.node_id.unwrap(),
        ))?;

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
                .sort(&Interval::end_n(), SortDir::Descend)
                .limit(1),
        )?;

        if interval.is_empty() {
            return Ok(None);
        }

        let interval = interval.first().unwrap();

        let node: Vec<Node> = self.get_by_filter(eq(
            Node::id_n(),
            interval.node_id.unwrap(),
        ))?;

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
            let node = self.find_path_part(&p, &parent)?;
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
                message: "provided value for path is empty"
                    .to_string(),
            });
        }

        let mut nodes = self.find_path(path)?;
        let p_len = path.len();
        let n_len = nodes.len();
        let high = p_len - (p_len - n_len);

        if high == p_len {
            return Ok(nodes);
        }

        let high = usize::try_from(high).unwrap();
        let mut parent_id = None;
        if !nodes.is_empty() {
            parent_id = Some(nodes.last().unwrap().id)
        }
        for n in path.iter().take(p_len).skip(high) {
            let mut node = Node {
                id: 0,
                parent_id,
                label: n.to_string(),
                created: Utc::now(),
                closed: false,
                deleted: false,
            };
            let id = self.save(&node)?;
            node.id = id;
            parent_id = Some(id);
            nodes.push(node);
        }

        Ok(nodes)
    }

    /// Returns ancestors of node with givent id, inluding
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

    fn get_filtered_forest(
        &self,
        filt: Filter,
    ) -> DBResult<Option<Forest<Node>>>
    where
        Self: Sized,
    {
        let filt = filter(filt)
            .sort(Node::parent_id_n(), SortDir::Ascend)
            .sort(Node::id_n(), SortDir::Ascend);

        let mut matched: Vec<Node> = self.get_by_statement(filt)?;
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
        node = if let Some(v) = try_add_child(tree, node) {
            v
        } else {
            return None;
        }
    }

    Some(node)
}
fn try_add_child(tree: PinNode, node: Node) -> Option<Node> {
    let mut tree = tree;
    if tree.data.id == node.parent_id.unwrap() {
        tree.push_back(tr(node));
        return None;
    } else {
        let mut node = node;
        for child in tree.iter_mut() {
            let res = try_add_child(child, node);
            if res.is_none() {
                return None;
            } else {
                node = res.unwrap();
            }
        }
        return Some(node);
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
        return None;
    }
}

#[derive(Debug, Clone, Identifiers)]
pub struct Node {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub label: String,
    pub created: DateTime<Utc>,
    pub closed: bool,
    pub deleted: bool,
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
        }
    }
}
impl ToString for Node {
    fn to_string(&self) -> String {
        self.label.to_owned()
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
