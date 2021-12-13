use super::FieldVal;

#[derive(Debug, Clone, Default)]
pub struct Statement<'a> {
  pub filter: Option<Filter<'a>>,
  pub sorts: Option<Vec<SortItem>>,
  pub limit: Option<usize>,
  pub offset: Option<usize>,
  pub distinct: bool,
  pub recursive_on: Option<&'a str>,
  pub from: Option<&'a str>,
  pub alias: Option<&'a str>,
}

impl<'a> Statement<'a> {
  pub fn filter(mut self, f: Filter<'a>) -> Self {
    self.filter = Some(f);
    self
  }
  pub fn sort(mut self, field: &str, direction: SortDir) -> Self {
    let mut sorts = self.sorts.unwrap_or_else(Vec::new);
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
  pub fn recursive_on(mut self, v: &'a str) -> Self {
    self.recursive_on = Some(v);
    self
  }
  pub fn from(mut self, v: &'a str) -> Self {
    self.from = Some(v);
    self
  }
  pub fn alias(mut self, v: &'a str) -> Self {
    self.alias = Some(v);
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
pub enum Filter<'a> {
  CmpOp(CmpOp<'a>),
  LogOp(Box<LogOp<'a>>),
  Exists(Box<Statement<'a>>),
}

#[derive(Debug, Clone)]
pub enum LogOp<'a> {
  And(Filter<'a>, Filter<'a>),
  Or(Filter<'a>, Filter<'a>),
  Not(Filter<'a>),
}

#[derive(Debug, Clone)]
pub enum CmpOp<'a> {
  Gt(&'a str, FieldVal),
  Lt(&'a str, FieldVal),
  Eq(&'a str, FieldVal),
  Ne(&'a str, FieldVal),
}
pub fn filter(v: Filter) -> Statement {
  Statement::default().filter(v)
}
pub fn sort(field: &str, direction: SortDir) -> Statement {
  Statement::default().sort(field, direction)
}
pub fn limit<'a>(v: usize) -> Statement<'a> {
  Statement::default().limit(v)
}
pub fn offset<'a>(v: usize) -> Statement<'a> {
  Statement::default().offset(v)
}
pub fn distinct<'a>() -> Statement<'a> {
  Statement::default().distinct()
}
pub fn from(v: &str) -> Statement {
  Statement::default().from(v)
}
pub fn gt(field: &str, value: impl Into<FieldVal>) -> Filter {
  Filter::CmpOp(CmpOp::Gt(field, value.into()))
}
pub fn lt(field: &str, value: impl Into<FieldVal>) -> Filter {
  Filter::CmpOp(CmpOp::Lt(field, value.into()))
}
pub fn eq(field: &str, value: impl Into<FieldVal>) -> Filter {
  Filter::CmpOp(CmpOp::Eq(field, value.into()))
}
pub fn ne(field: &str, value: impl Into<FieldVal>) -> Filter {
  Filter::CmpOp(CmpOp::Ne(field, value.into()))
}
pub fn and<'a>(f1: Filter<'a>, f2: Filter<'a>) -> Filter<'a> {
  Filter::LogOp(Box::new(LogOp::And(f1, f2)))
}
pub fn or<'a>(f1: Filter<'a>, f2: Filter<'a>) -> Filter<'a> {
  Filter::LogOp(Box::new(LogOp::Or(f1, f2)))
}
pub fn exists(s: Statement) -> Filter {
  Filter::Exists(Box::new(s))
}
pub fn not(f: Filter) -> Filter {
  Filter::LogOp(Box::new(LogOp::Not(f)))
}
