mod core {
  use chrono::prelude::*;
  use std::cmp::*;
  use std::convert::TryFrom;
  use std::error::Error;
  use trees::{tr, Forest, ForestWalk, Visit};
  use yatt_orm::errors::{DBError, DBResult};
  use yatt_orm::sqlite::DB;
  use yatt_orm::statement::*;
  use yatt_orm::{FieldVal, Identifiers, Storage};
  type PinNode<'a> = std::pin::Pin<&'a mut trees::Node<Node>>;
  pub(crate) struct ImportedPath {
    nodes: Vec<Node>,
    source: String,
    comment: String,
  }
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::fmt::Debug for ImportedPath {
    fn fmt(
      &self,
      f: &mut ::core::fmt::Formatter,
    ) -> ::core::fmt::Result {
      match *self {
        ImportedPath {
          nodes: ref __self_0_0,
          source: ref __self_0_1,
          comment: ref __self_0_2,
        } => {
          let debug_trait_builder =
            &mut ::core::fmt::Formatter::debug_struct(
              f,
              "ImportedPath",
            );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "nodes",
            &&(*__self_0_0),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "source",
            &&(*__self_0_1),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "comment",
            &&(*__self_0_2),
          );
          ::core::fmt::DebugStruct::finish(debug_trait_builder)
        }
      }
    }
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
          message: {
            let res =
              ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                &[
                  "Task with id=",
                  " for interval with id=",
                  ", not exists",
                ],
                &match (&interval.node_id.unwrap_or(0), &interval.id)
                {
                  _args => [
                    ::core::fmt::ArgumentV1::new(
                      _args.0,
                      ::core::fmt::Display::fmt,
                    ),
                    ::core::fmt::ArgumentV1::new(
                      _args.1,
                      ::core::fmt::Display::fmt,
                    ),
                  ],
                },
              ));
            res
          },
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
      let node: Vec<Node> = self
        .get_by_filter(eq(Node::id_n(), interval.node_id.unwrap()))?;
      if node.is_empty() {
        return Err(DBError::Unexpected {
          message: {
            let res =
              ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                &[
                  "Task with id=",
                  " for interval with id=",
                  ", not exists",
                ],
                &match (&interval.node_id.unwrap_or(0), &interval.id)
                {
                  _args => [
                    ::core::fmt::ArgumentV1::new(
                      _args.0,
                      ::core::fmt::Display::fmt,
                    ),
                    ::core::fmt::ArgumentV1::new(
                      _args.1,
                      ::core::fmt::Display::fmt,
                    ),
                  ],
                },
              ));
            res
          },
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
      FlattenForestIter {
        walk,
        path: ::alloc::vec::Vec::new(),
      }
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
  pub struct Node {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub label: String,
    pub created: DateTime<Utc>,
    pub closed: bool,
    pub deleted: bool,
  }
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::fmt::Debug for Node {
    fn fmt(
      &self,
      f: &mut ::core::fmt::Formatter,
    ) -> ::core::fmt::Result {
      match *self {
        Node {
          id: ref __self_0_0,
          parent_id: ref __self_0_1,
          label: ref __self_0_2,
          created: ref __self_0_3,
          closed: ref __self_0_4,
          deleted: ref __self_0_5,
        } => {
          let debug_trait_builder =
            &mut ::core::fmt::Formatter::debug_struct(f, "Node");
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "id",
            &&(*__self_0_0),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "parent_id",
            &&(*__self_0_1),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "label",
            &&(*__self_0_2),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "created",
            &&(*__self_0_3),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "closed",
            &&(*__self_0_4),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "deleted",
            &&(*__self_0_5),
          );
          ::core::fmt::DebugStruct::finish(debug_trait_builder)
        }
      }
    }
  }
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::clone::Clone for Node {
    #[inline]
    fn clone(&self) -> Node {
      match *self {
        Node {
          id: ref __self_0_0,
          parent_id: ref __self_0_1,
          label: ref __self_0_2,
          created: ref __self_0_3,
          closed: ref __self_0_4,
          deleted: ref __self_0_5,
        } => Node {
          id: ::core::clone::Clone::clone(&(*__self_0_0)),
          parent_id: ::core::clone::Clone::clone(&(*__self_0_1)),
          label: ::core::clone::Clone::clone(&(*__self_0_2)),
          created: ::core::clone::Clone::clone(&(*__self_0_3)),
          closed: ::core::clone::Clone::clone(&(*__self_0_4)),
          deleted: ::core::clone::Clone::clone(&(*__self_0_5)),
        },
      }
    }
  }
  impl Node {
    const ID_N_CONST: &'static str = &"id";
    pub fn id_n() -> &'static str {
      Self::ID_N_CONST
    }
    const PARENT_ID_N_CONST: &'static str = &"parent_id";
    pub fn parent_id_n() -> &'static str {
      Self::PARENT_ID_N_CONST
    }
    const LABEL_N_CONST: &'static str = &"label";
    pub fn label_n() -> &'static str {
      Self::LABEL_N_CONST
    }
    const CREATED_N_CONST: &'static str = &"created";
    pub fn created_n() -> &'static str {
      Self::CREATED_N_CONST
    }
    const CLOSED_N_CONST: &'static str = &"closed";
    pub fn closed_n() -> &'static str {
      Self::CLOSED_N_CONST
    }
    const DELETED_N_CONST: &'static str = &"deleted";
    pub fn deleted_n() -> &'static str {
      Self::DELETED_N_CONST
    }
  }
  impl Node {
    const STRUCT_NAME: &'static str = Node;
    const FIELD_LIST: &'static [&'static str] = &[
      &"id",
      &"parent_id",
      &"label",
      &"created",
      &"closed",
      &"deleted",
    ];
  }
  impl yatt_orm::StoreObject for Node {
    fn get_type_name(&self) -> &'static str {
      Self::STRUCT_NAME
    }
    fn get_field_val(&self, field_name: &str) -> yatt_orm::FieldVal {
      match field_name {
        "id" => self.id.clone().into(),
        "parent_id" => self.parent_id.clone().into(),
        "label" => self.label.clone().into(),
        "created" => self.created.clone().into(),
        "closed" => self.closed.clone().into(),
        "deleted" => self.deleted.clone().into(),
        _ => ::core::panicking::panic_display(&{
          let res =
            ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
              &["there is no field ", " in struct "],
              &match (&field_name, &"Node") {
                _args => [
                  ::core::fmt::ArgumentV1::new(
                    _args.0,
                    ::core::fmt::Display::fmt,
                  ),
                  ::core::fmt::ArgumentV1::new(
                    _args.1,
                    ::core::fmt::Display::fmt,
                  ),
                ],
              },
            ));
          res
        }),
      }
    }
    fn set_field_val(
      &mut self,
      field_name: &str,
      val: impl Into<yatt_orm::FieldVal>,
    ) -> yatt_orm::DBResult<()> {
      let val: yatt_orm::FieldVal = val.into();
      match field_name {
        "id" => self.id = std::convert::TryInto::try_into(val)?,
        "parent_id" => {
          self.parent_id = std::convert::TryInto::try_into(val)?
        }
        "label" => self.label = std::convert::TryInto::try_into(val)?,
        "created" => {
          self.created = std::convert::TryInto::try_into(val)?
        }
        "closed" => {
          self.closed = std::convert::TryInto::try_into(val)?
        }
        "deleted" => {
          self.deleted = std::convert::TryInto::try_into(val)?
        }
        _ => ::core::panicking::panic_display(&{
          let res =
            ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
              &["there is no field ", " in struct "],
              &match (&field_name, &"Node") {
                _args => [
                  ::core::fmt::ArgumentV1::new(
                    _args.0,
                    ::core::fmt::Display::fmt,
                  ),
                  ::core::fmt::ArgumentV1::new(
                    _args.1,
                    ::core::fmt::Display::fmt,
                  ),
                ],
              },
            ));
          res
        }),
      }
      Ok(())
    }
    fn get_fields_list(&self) -> &'static [&'static str] {
      Self::FIELD_LIST
    }
  }
  impl ::core::marker::StructuralPartialEq for Node {}
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::cmp::PartialEq for Node {
    #[inline]
    fn eq(&self, other: &Node) -> bool {
      match *other {
        Node {
          id: ref __self_1_0,
          parent_id: ref __self_1_1,
          label: ref __self_1_2,
          created: ref __self_1_3,
          closed: ref __self_1_4,
          deleted: ref __self_1_5,
        } => match *self {
          Node {
            id: ref __self_0_0,
            parent_id: ref __self_0_1,
            label: ref __self_0_2,
            created: ref __self_0_3,
            closed: ref __self_0_4,
            deleted: ref __self_0_5,
          } => {
            (*__self_0_0) == (*__self_1_0)
              && (*__self_0_1) == (*__self_1_1)
              && (*__self_0_2) == (*__self_1_2)
              && (*__self_0_3) == (*__self_1_3)
              && (*__self_0_4) == (*__self_1_4)
              && (*__self_0_5) == (*__self_1_5)
          }
        },
      }
    }
    #[inline]
    fn ne(&self, other: &Node) -> bool {
      match *other {
        Node {
          id: ref __self_1_0,
          parent_id: ref __self_1_1,
          label: ref __self_1_2,
          created: ref __self_1_3,
          closed: ref __self_1_4,
          deleted: ref __self_1_5,
        } => match *self {
          Node {
            id: ref __self_0_0,
            parent_id: ref __self_0_1,
            label: ref __self_0_2,
            created: ref __self_0_3,
            closed: ref __self_0_4,
            deleted: ref __self_0_5,
          } => {
            (*__self_0_0) != (*__self_1_0)
              || (*__self_0_1) != (*__self_1_1)
              || (*__self_0_2) != (*__self_1_2)
              || (*__self_0_3) != (*__self_1_3)
              || (*__self_0_4) != (*__self_1_4)
              || (*__self_0_5) != (*__self_1_5)
          }
        },
      }
    }
  }
  impl ::core::marker::StructuralEq for Node {}
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::cmp::Eq for Node {
    #[inline]
    #[doc(hidden)]
    #[no_coverage]
    fn assert_receiver_is_total_eq(&self) -> () {
      {
        let _: ::core::cmp::AssertParamIsEq<usize>;
        let _: ::core::cmp::AssertParamIsEq<Option<usize>>;
        let _: ::core::cmp::AssertParamIsEq<String>;
        let _: ::core::cmp::AssertParamIsEq<DateTime<Utc>>;
        let _: ::core::cmp::AssertParamIsEq<bool>;
        let _: ::core::cmp::AssertParamIsEq<bool>;
      }
    }
  }
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::cmp::Ord for Node {
    #[inline]
    fn cmp(&self, other: &Node) -> ::core::cmp::Ordering {
      match *other {
        Node {
          id: ref __self_1_0,
          parent_id: ref __self_1_1,
          label: ref __self_1_2,
          created: ref __self_1_3,
          closed: ref __self_1_4,
          deleted: ref __self_1_5,
        } => match *self {
          Node {
            id: ref __self_0_0,
            parent_id: ref __self_0_1,
            label: ref __self_0_2,
            created: ref __self_0_3,
            closed: ref __self_0_4,
            deleted: ref __self_0_5,
          } => match ::core::cmp::Ord::cmp(
            &(*__self_0_0),
            &(*__self_1_0),
          ) {
            ::core::cmp::Ordering::Equal => {
              match ::core::cmp::Ord::cmp(
                &(*__self_0_1),
                &(*__self_1_1),
              ) {
                ::core::cmp::Ordering::Equal => {
                  match ::core::cmp::Ord::cmp(
                    &(*__self_0_2),
                    &(*__self_1_2),
                  ) {
                    ::core::cmp::Ordering::Equal => {
                      match ::core::cmp::Ord::cmp(
                        &(*__self_0_3),
                        &(*__self_1_3),
                      ) {
                        ::core::cmp::Ordering::Equal => {
                          match ::core::cmp::Ord::cmp(
                            &(*__self_0_4),
                            &(*__self_1_4),
                          ) {
                            ::core::cmp::Ordering::Equal => {
                              match ::core::cmp::Ord::cmp(
                                &(*__self_0_5),
                                &(*__self_1_5),
                              ) {
                                ::core::cmp::Ordering::Equal => {
                                  ::core::cmp::Ordering::Equal
                                }
                                cmp => cmp,
                              }
                            }
                            cmp => cmp,
                          }
                        }
                        cmp => cmp,
                      }
                    }
                    cmp => cmp,
                  }
                }
                cmp => cmp,
              }
            }
            cmp => cmp,
          },
        },
      }
    }
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
      }
    }
  }
  impl ToString for Node {
    fn to_string(&self) -> String {
      self.label.to_owned()
    }
  }
  pub struct Interval {
    pub id: usize,
    pub node_id: Option<usize>,
    pub begin: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub deleted: bool,
    pub closed: bool,
  }
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::fmt::Debug for Interval {
    fn fmt(
      &self,
      f: &mut ::core::fmt::Formatter,
    ) -> ::core::fmt::Result {
      match *self {
        Interval {
          id: ref __self_0_0,
          node_id: ref __self_0_1,
          begin: ref __self_0_2,
          end: ref __self_0_3,
          deleted: ref __self_0_4,
          closed: ref __self_0_5,
        } => {
          let debug_trait_builder =
            &mut ::core::fmt::Formatter::debug_struct(f, "Interval");
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "id",
            &&(*__self_0_0),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "node_id",
            &&(*__self_0_1),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "begin",
            &&(*__self_0_2),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "end",
            &&(*__self_0_3),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "deleted",
            &&(*__self_0_4),
          );
          let _ = ::core::fmt::DebugStruct::field(
            debug_trait_builder,
            "closed",
            &&(*__self_0_5),
          );
          ::core::fmt::DebugStruct::finish(debug_trait_builder)
        }
      }
    }
  }
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::clone::Clone for Interval {
    #[inline]
    fn clone(&self) -> Interval {
      {
        let _: ::core::clone::AssertParamIsClone<usize>;
        let _: ::core::clone::AssertParamIsClone<Option<usize>>;
        let _: ::core::clone::AssertParamIsClone<DateTime<Utc>>;
        let _: ::core::clone::AssertParamIsClone<
          Option<DateTime<Utc>>,
        >;
        let _: ::core::clone::AssertParamIsClone<bool>;
        let _: ::core::clone::AssertParamIsClone<bool>;
        *self
      }
    }
  }
  #[automatically_derived]
  #[allow(unused_qualifications)]
  impl ::core::marker::Copy for Interval {}
  impl Interval {
    const ID_N_CONST: &'static str = &"id";
    pub fn id_n() -> &'static str {
      Self::ID_N_CONST
    }
    const NODE_ID_N_CONST: &'static str = &"node_id";
    pub fn node_id_n() -> &'static str {
      Self::NODE_ID_N_CONST
    }
    const BEGIN_N_CONST: &'static str = &"begin";
    pub fn begin_n() -> &'static str {
      Self::BEGIN_N_CONST
    }
    const END_N_CONST: &'static str = &"end";
    pub fn end_n() -> &'static str {
      Self::END_N_CONST
    }
    const DELETED_N_CONST: &'static str = &"deleted";
    pub fn deleted_n() -> &'static str {
      Self::DELETED_N_CONST
    }
    const CLOSED_N_CONST: &'static str = &"closed";
    pub fn closed_n() -> &'static str {
      Self::CLOSED_N_CONST
    }
  }
  impl Interval {
    const STRUCT_NAME: &'static str = Interval;
    const FIELD_LIST: &'static [&'static str] =
      &[&"id", &"node_id", &"begin", &"end", &"deleted", &"closed"];
  }
  impl yatt_orm::StoreObject for Interval {
    fn get_type_name(&self) -> &'static str {
      Self::STRUCT_NAME
    }
    fn get_field_val(&self, field_name: &str) -> yatt_orm::FieldVal {
      match field_name {
        "id" => self.id.clone().into(),
        "node_id" => self.node_id.clone().into(),
        "begin" => self.begin.clone().into(),
        "end" => self.end.clone().into(),
        "deleted" => self.deleted.clone().into(),
        "closed" => self.closed.clone().into(),
        _ => ::core::panicking::panic_display(&{
          let res =
            ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
              &["there is no field ", " in struct "],
              &match (&field_name, &"Interval") {
                _args => [
                  ::core::fmt::ArgumentV1::new(
                    _args.0,
                    ::core::fmt::Display::fmt,
                  ),
                  ::core::fmt::ArgumentV1::new(
                    _args.1,
                    ::core::fmt::Display::fmt,
                  ),
                ],
              },
            ));
          res
        }),
      }
    }
    fn set_field_val(
      &mut self,
      field_name: &str,
      val: impl Into<yatt_orm::FieldVal>,
    ) -> yatt_orm::DBResult<()> {
      let val: yatt_orm::FieldVal = val.into();
      match field_name {
        "id" => self.id = std::convert::TryInto::try_into(val)?,
        "node_id" => {
          self.node_id = std::convert::TryInto::try_into(val)?
        }
        "begin" => self.begin = std::convert::TryInto::try_into(val)?,
        "end" => self.end = std::convert::TryInto::try_into(val)?,
        "deleted" => {
          self.deleted = std::convert::TryInto::try_into(val)?
        }
        "closed" => {
          self.closed = std::convert::TryInto::try_into(val)?
        }
        _ => ::core::panicking::panic_display(&{
          let res =
            ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
              &["there is no field ", " in struct "],
              &match (&field_name, &"Interval") {
                _args => [
                  ::core::fmt::ArgumentV1::new(
                    _args.0,
                    ::core::fmt::Display::fmt,
                  ),
                  ::core::fmt::ArgumentV1::new(
                    _args.1,
                    ::core::fmt::Display::fmt,
                  ),
                ],
              },
            ));
          res
        }),
      }
      Ok(())
    }
    fn get_fields_list(&self) -> &'static [&'static str] {
      Self::FIELD_LIST
    }
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
      {
        let res =
          ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
            &["[started: ", " stopped: ", "]"],
            &match (&self.begin, &end) {
              _args => [
                ::core::fmt::ArgumentV1::new(
                  _args.0,
                  ::core::fmt::Display::fmt,
                ),
                ::core::fmt::ArgumentV1::new(
                  _args.1,
                  ::core::fmt::Display::fmt,
                ),
              ],
            },
          ));
        res
      }
    }
  }
  impl DBRoot for DB<'_> {}
}
