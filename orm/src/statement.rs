use chrono::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct Statement {
    pub filter: Option<Filter>,
    pub sorts: Option<Vec<SortItem>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub distinct: bool,
}

impl Statement {
    pub fn filter(mut self, f: Filter) -> Self {
        self.filter = Some(f);
        self
    }
    pub fn sort(mut self, field: &str, direction: SortDir) -> Self {
        let mut sorts = self.sorts.unwrap_or_else(|| vec![]);
        sorts.push(SortItem(field.into(), direction));
        self.sorts = Some(sorts);
        self
    }
    pub fn limit(mut self, v: usize) -> Self {
        self.limit = Some(v);
        self
    }
    pub fn offset(mut self, v: usize) -> Self {
        self.offset = Some(v);
        self
    }
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }
}

#[derive(Debug, Clone)]
pub enum SortDir {
    Ascend,
    Descend,
}

#[derive(Debug, Clone)]
pub struct SortItem(pub String, pub SortDir);

#[derive(Debug, Clone)]
pub enum CmpVal {
    Usize(usize),
    DateTime(DateTime<Utc>),
    String(String),
    Null,
}

#[derive(Debug, Clone)]
pub enum Filter {
    CmpOp(CmpOp),
    LogOp(Box<LogOp>),
}

#[derive(Debug, Clone)]
pub enum LogOp {
    And(Filter, Filter),
    Or(Filter, Filter),
    Not(Filter),
}

#[derive(Debug, Clone)]
pub enum CmpOp {
    Gt(String, CmpVal),
    Lt(String, CmpVal),
    Eq(String, CmpVal),
    Ne(String, CmpVal),
}
pub fn filter(v: Filter) -> Statement {
    Statement::default().filter(v)
}
pub fn sort(field: &str, direction: SortDir) -> Statement {
    Statement::default().sort(field, direction)
}
pub fn limit(v: usize) -> Statement {
    Statement::default().limit(v)
}
pub fn offset(v: usize) -> Statement {
    Statement::default().offset(v)
}
pub fn distinct() -> Statement {
    Statement::default().distinct()
}
pub fn gt(field: String, value: impl Into<CmpVal>) -> Filter {
    Filter::CmpOp(CmpOp::Gt(field, value.into()))
}
pub fn lt(field: String, value: impl Into<CmpVal>) -> Filter {
    Filter::CmpOp(CmpOp::Lt(field, value.into()))
}
pub fn eq(field: String, value: impl Into<CmpVal>) -> Filter {
    Filter::CmpOp(CmpOp::Eq(field, value.into()))
}
pub fn ne(field: String, value: impl Into<CmpVal>) -> Filter {
    Filter::CmpOp(CmpOp::Ne(field, value.into()))
}
pub fn and(f1: Filter, f2: Filter) -> Filter {
    Filter::LogOp(Box::new(LogOp::And(f1, f2)))
}
pub fn or(f1: Filter, f2: Filter) -> Filter {
    Filter::LogOp(Box::new(LogOp::Or(f1, f2)))
}
pub fn not(f: Filter) -> Filter {
    Filter::LogOp(Box::new(LogOp::Not(f)))
}

impl From<usize> for CmpVal {
    fn from(u: usize) -> CmpVal {
        CmpVal::Usize(u)
    }
}
impl From<DateTime<Local>> for CmpVal {
    fn from(val: DateTime<Local>) -> CmpVal {
        CmpVal::DateTime(DateTime::from(val))
    }
}
impl From<DateTime<Utc>> for CmpVal {
    fn from(val: DateTime<Utc>) -> CmpVal {
        CmpVal::DateTime(val)
    }
}
impl From<&str> for CmpVal {
    fn from(val: &str) -> CmpVal {
        CmpVal::String(val.to_string())
    }
}
impl From<String> for CmpVal {
    fn from(val: String) -> CmpVal {
        CmpVal::String(val)
    }
}
impl From<&String> for CmpVal {
    fn from(val: &String) -> CmpVal {
        CmpVal::String(val.clone())
    }
}
impl From<&CmpVal> for CmpVal {
    fn from(val: &CmpVal) -> CmpVal {
        (*val).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let s = String::from("a");
        let g = gt(s, 8);
        assert_eq!(2 + 2, 4);
    }
}
