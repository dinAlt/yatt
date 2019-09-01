#![feature(prelude_import)]
#![no_std]
#[prelude_import]
use ::std::prelude::v1::*;
#[macro_use]
extern crate std;
use std::convert::TryFrom;

use chrono::prelude::*;

use orm::errors::{DBError, DBResult};
use orm::statement::*;
use orm::{BoxStorage, Identifiers, FieldList, Fieldlist};

pub trait DBRoot {
    fn nodes(&self)
    -> BoxStorage<Node>;
    fn intervals(&self)
    -> BoxStorage<Interval>;
}

impl dyn DBRoot {
    pub fn cur_running(&self) -> DBResult<Option<(Node, Interval)>> {
        let intrval =
            self.intervals().filter(eq(Interval::end_n(), CmpVal::Null))?;

        if intrval.is_empty() { return Ok(None); }

        let interval = intrval[0].clone();

        let node =
            self.nodes().filter(eq(Node::id_n(), interval.node_id.unwrap()))?;

        if node.is_empty() {
            return Err(DBError::Unexpected{message:























                                               ::alloc::fmt::format(::core::fmt::Arguments::new_v1(&["Task with id=",
                                                                                                     " for interval with id=",
                                                                                                     ", not exists"],
                                                                                                   &match (&interval.node_id.unwrap_or(0),
                                                                                                           &interval.id)
                                                                                                        {
                                                                                                        (arg0,
                                                                                                         arg1)
                                                                                                        =>
                                                                                                        [::core::fmt::ArgumentV1::new(arg0,
                                                                                                                                      ::core::fmt::Display::fmt),
                                                                                                         ::core::fmt::ArgumentV1::new(arg1,
                                                                                                                                      ::core::fmt::Display::fmt)],
                                                                                                    })),});
        }
        let node = node[0].clone();
        Ok(Some((node, interval)))
    }
    pub fn last_running(&self) -> DBResult<Option<(Node, Interval)>> {
        let interval = self.intervals().with_max("end")?;
        if interval.is_none() { return Ok(None); }
        let interval = interval.unwrap();
        let node =
            self.nodes().filter(eq(Node::id_n(), interval.node_id.unwrap()))?;
        if node.is_empty() {
            return Err(DBError::Unexpected{message:
                                               ::alloc::fmt::format(::core::fmt::Arguments::new_v1(&["Task with id=",
                                                                                                     " for interval with id=",
                                                                                                     ", not exists"],
                                                                                                   &match (&interval.node_id.unwrap_or(0),
                                                                                                           &interval.id)
                                                                                                        {
                                                                                                        (arg0,
                                                                                                         arg1)
                                                                                                        =>
                                                                                                        [::core::fmt::ArgumentV1::new(arg0,
                                                                                                                                      ::core::fmt::Display::fmt),
                                                                                                         ::core::fmt::ArgumentV1::new(arg1,
                                                                                                                                      ::core::fmt::Display::fmt)],
                                                                                                    })),});
        }
        let node = node[0].clone();
        Ok(Some((node, interval)))
    }
    pub fn find_path(&self, path: &[&str]) -> DBResult<Vec<Node>> {
        let mut parent = CmpVal::Null;
        let mut res = Vec::new();
        for p in path.iter() {
            let node = self.find_path_part(&p, &parent)?;
            if let Some(node) = node {
                parent = CmpVal::Usize(node.id);
                res.push(node);
            } else { return Ok(res); }
        }
        Ok(res)
    }
    /// Crates all not exist node of path, and returns all nodes.
    pub fn create_path(&self, path: &[&str]) -> DBResult<Vec<Node>> {
        if path.is_empty() {
            return Err(DBError::Unexpected{message:
                                               "provided value for path is empty".to_string(),});
        }
        let mut nodes = self.find_path(path)?;
        let p_len = path.len();
        let n_len = nodes.len();
        let high = p_len - (p_len - n_len);
        if high == p_len { return Ok(nodes); }
        let high = usize::try_from(high).unwrap();
        let mut parent_id = None;
        if !nodes.is_empty() { parent_id = Some(nodes.last().unwrap().id) }
        let mut node;
        for n in path.iter().take(p_len).skip(high) {
            node =
                Node{id: 0,
                     parent_id,
                     label: n.to_string(),
                     created: Utc::now(),
                     closed: false,
                     deleted: false,};
            let id = self.nodes().save(&node)?;
            node.id = id;
            parent_id = Some(id);
            nodes.push(node.clone());
        }
        Ok(nodes)
    }
    /// Returns ancestors of node with givent id, inluding
    /// the node with given id itself.
    pub fn ancestors(&self, id: usize) -> DBResult<Vec<Node>> {
        let mut res = Vec::new();
        let mut next = Some(id);
        while next.is_some() {
            let node = self.nodes().by_id(next.unwrap())?;
            next = node.parent_id;
            res.push(node);
        }
        res.reverse();
        Ok(res)
    }
    fn find_path_part(&self, name: &str, parent_id: &CmpVal)
     -> DBResult<Option<Node>> {
        let nodes =
            self.nodes().filter(and(eq(Node::parent_id_n(), parent_id),
                                    eq(Node::label_n(), name)))?;
        if nodes.is_empty() { return Ok(None); };
        if nodes.len() > 1 {
            return Err(DBError::Unexpected{message:
                                               "query return multiple rows".to_string(),});
        };
        Ok(Some(nodes[0].clone()))
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
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Node {
            id: ref __self_0_0,
            parent_id: ref __self_0_1,
            label: ref __self_0_2,
            created: ref __self_0_3,
            closed: ref __self_0_4,
            deleted: ref __self_0_5 } => {
                let mut debug_trait_builder = f.debug_struct("Node");
                let _ = debug_trait_builder.field("id", &&(*__self_0_0));
                let _ =
                    debug_trait_builder.field("parent_id", &&(*__self_0_1));
                let _ = debug_trait_builder.field("label", &&(*__self_0_2));
                let _ = debug_trait_builder.field("created", &&(*__self_0_3));
                let _ = debug_trait_builder.field("closed", &&(*__self_0_4));
                let _ = debug_trait_builder.field("deleted", &&(*__self_0_5));
                debug_trait_builder.finish()
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
            deleted: ref __self_0_5 } =>
            Node{id: ::core::clone::Clone::clone(&(*__self_0_0)),
                 parent_id: ::core::clone::Clone::clone(&(*__self_0_1)),
                 label: ::core::clone::Clone::clone(&(*__self_0_2)),
                 created: ::core::clone::Clone::clone(&(*__self_0_3)),
                 closed: ::core::clone::Clone::clone(&(*__self_0_4)),
                 deleted: ::core::clone::Clone::clone(&(*__self_0_5)),},
        }
    }
}
impl ToString for Node {
    fn to_string(&self) -> String { self.label.to_owned() }
}
pub struct Interval {
    pub id: usize,
    pub node_id: Option<usize>,
    pub begin: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub deleted: bool,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::fmt::Debug for Interval {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match *self {
            Interval {
            id: ref __self_0_0,
            node_id: ref __self_0_1,
            begin: ref __self_0_2,
            end: ref __self_0_3,
            deleted: ref __self_0_4 } => {
                let mut debug_trait_builder = f.debug_struct("Interval");
                let _ = debug_trait_builder.field("id", &&(*__self_0_0));
                let _ = debug_trait_builder.field("node_id", &&(*__self_0_1));
                let _ = debug_trait_builder.field("begin", &&(*__self_0_2));
                let _ = debug_trait_builder.field("end", &&(*__self_0_3));
                let _ = debug_trait_builder.field("deleted", &&(*__self_0_4));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::core::clone::Clone for Interval {
    #[inline]
    fn clone(&self) -> Interval {
        match *self {
            Interval {
            id: ref __self_0_0,
            node_id: ref __self_0_1,
            begin: ref __self_0_2,
            end: ref __self_0_3,
            deleted: ref __self_0_4 } =>
            Interval{id: ::core::clone::Clone::clone(&(*__self_0_0)),
                     node_id: ::core::clone::Clone::clone(&(*__self_0_1)),
                     begin: ::core::clone::Clone::clone(&(*__self_0_2)),
                     end: ::core::clone::Clone::clone(&(*__self_0_3)),
                     deleted: ::core::clone::Clone::clone(&(*__self_0_4)),},
        }
    }
}
impl ToString for Interval {
    fn to_string(&self) -> String {
        let end =
            match self.end {
                Some(d) => d.to_rfc3339(),
                None => "never".to_string(),
            };
        ::alloc::fmt::format(::core::fmt::Arguments::new_v1(&["[started: ",
                                                              " stopped: ",
                                                              "]"],
                                                            &match (&self.begin,
                                                                    &end) {
                                                                 (arg0, arg1)
                                                                 =>
                                                                 [::core::fmt::ArgumentV1::new(arg0,
                                                                                               ::core::fmt::Display::fmt),
                                                                  ::core::fmt::ArgumentV1::new(arg1,
                                                                                               ::core::fmt::Display::fmt)],
                                                             }))
    }
}
