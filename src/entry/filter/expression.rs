
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Logic(Box<Expr>, ELogicOp, Box<Expr>),
    Boolean(EBool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EBool {
    Compare(EValue, ECompOp, EValue),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ECompOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ELogicOp {
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EValue {
    Integer(i64),
    Variable(EVariable),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EVariable {
    Width,
    Height
}
