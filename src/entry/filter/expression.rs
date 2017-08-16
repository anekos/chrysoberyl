
use std::default::Default;

use globset::GlobMatcher;



#[derive(Clone, Debug)]
pub enum Expr {
    Logic(Box<Expr>, ELogicOp, Box<Expr>),
    Boolean(EBool),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    When(bool,Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

#[derive(Clone, Debug)]
pub enum EBool {
    Compare(EValue, ECompOp, EValue),
    Variable(EBVariable),
    Resolution(i64, i64),
    True,
    False,
}

#[derive(Clone, Debug)]
pub enum ECompOp {
    ForInt(EICompOp),
    GlobMatch(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EICompOp {
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
    Eq
}

#[derive(Clone, Debug)]
pub enum ELogicOp {
    And,
    Or,
}

#[derive(Clone, Debug)]
pub enum EValue {
    Integer(i64),
    Variable(EVariable),
    Glob(Vec<(GlobMatcher, String)>),
}

#[derive(Clone, Debug, Copy)]
pub enum EVariable {
    Width,
    Height,
    Path,
    Name,
    Page,
    Extension,
    Type,
    Dimentions,
    FileSize,
}

#[derive(Clone, Debug, Copy)]
pub enum EBVariable {
    Animation,
}


impl Default for Expr {
    fn default() -> Self {
        Expr::Boolean(EBool::True)
    }
}

impl Expr {
    pub fn not(self) -> Self {
        Expr::Not(Box::new(self))
    }
}
