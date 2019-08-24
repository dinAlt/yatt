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

impl From<usize> for CmpVal {
    fn from(u: usize) -> CmpVal {
        CmpVal::Usize(u)
    }
}

pub trait AsCmpVal {
    fn as_cmp_val(self) -> CmpVal;
}

pub fn gt(field: String, value: impl AsCmpVal) -> Filter {
    Filter::CmpOp(CmpOp::Gt(field, value.as_cmp_val()))
}

impl AsCmpVal for usize {
    fn as_cmp_val(self) -> CmpVal {
        CmpVal::Usize(self)
    }
}

impl AsCmpVal for DateTime<Utc> {
    fn as_cmp_val(self) -> CmpVal {
        CmpVal::DateTime(self)
    }
}

impl AsCmpVal for DateTime<Local> {
    fn as_cmp_val(self) -> CmpVal {
        let u = DateTime::from(self);
        CmpVal::DateTime(u)
    }
}

impl AsCmpVal for String {
    fn as_cmp_val(self) -> CmpVal {
        CmpVal::String(self)
    }
}

impl AsCmpVal for &str {
    fn as_cmp_val(self) -> CmpVal {
        CmpVal::String(self.to_string())
    }
}

impl AsCmpVal for CmpVal {
    fn as_cmp_val(self) -> CmpVal {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        let s = String::from("a");
        let g = gt(s, "sdf");
        assert_eq!(2 + 2, 4);
    }
}
