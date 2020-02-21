use std::convert::TryFrom;

use chrono::prelude::*;

use yatt_orm::errors::{DBError, DBResult};
use yatt_orm::statement::*;
use yatt_orm::FieldVal;
use yatt_orm::{Identifiers, Storage};

pub trait DBRoot: Storage {
    fn cur_running(&self) -> DBResult<Option<(Node, Interval)>>
    where
        Self: Sized,
    {
        let interval: Vec<Interval> = self.get_by_filter(eq(Interval::end_n(), FieldVal::Null))?;

        if interval.is_empty() {
            return Ok(None);
        }

        let interval = interval[0].clone();

        let node: Vec<Node> = self.get_by_filter(eq(Node::id_n(), interval.node_id.unwrap()))?;

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

        let node: Vec<Node> = self.get_by_filter(eq(Node::id_n(), interval.node_id.unwrap()))?;

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
                message: "provided value for path is empty".to_string(),
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
        let mut node;
        for n in path.iter().take(p_len).skip(high) {
            node = Node {
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
            nodes.push(node.clone());
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

    fn find_path_part(&self, name: &str, parent_id: &FieldVal) -> DBResult<Option<Node>>
    where
        Self: Sized,
    {
        let nodes: Vec<Node> = self.get_by_filter(and(
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
        Ok(Some(nodes[0].clone()))
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
