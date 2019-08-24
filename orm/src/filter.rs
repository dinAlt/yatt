use chrono::prelude::*;

#[derive(Debug)]
pub enum CmpVal {
    Usize(usize),
    DateTime(DateTime<Utc>),
    String(String),
    Null,
}

#[derive(Debug)]
pub struct LogOp(Filter, Filter);

#[derive(Debug)]
pub enum Filter {
    CmpOp(CmpOp),
    BinOp(Box<LogOp>),
}

#[derive(Debug)]
pub enum CmpOp {
    Gt(String, CmpVal),
    Lt(String, CmpVal),
    Eq(String, CmpVal),
    Ne(String, CmpVal),
}

pub trait Identifier {
    fn string_value(&self) -> String;
}

impl From<usize> for CmpVal {
    fn from(u: usize) -> CmpVal {
        CmpVal::Usize(u)
    }
}

pub trait AsCmpVal {
    fn as_cmp_val(self) -> CmpVal;
}

pub fn gt(field: impl Identifier, value: impl AsCmpVal) -> Filter {
    Filter::CmpOp(CmpOp::Gt(field.string_value(), value.as_cmp_val()))
}

impl AsCmpVal for usize {
    fn as_cmp_val(self) -> CmpVal {
        CmpVal::Usize(self)
    }
}

impl AsCmpVal for CmpVal {
    fn as_cmp_val(self) -> CmpVal {
        self
    }
}
