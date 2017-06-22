
use pom::{Parser, DataInput};
use pom::parser::*;

use std::str::FromStr;



#[derive(Debug, PartialEq)]
pub enum Expr {
    Logic(Box<Expr>, ELogicOp, Box<Expr>),
    Boolean(EBool),
}

#[derive(Debug, PartialEq)]
pub enum EBool {
    Compare(EValue, ECompOp, EValue),
}

#[derive(Debug, PartialEq)]
pub enum ECompOp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    Ne,
}

#[derive(Debug, PartialEq)]
pub enum ELogicOp {
    And,
    Or,
}

#[derive(Debug, PartialEq)]
pub enum EValue {
    Integer(i64),
    Variable(EVariable),
}

#[derive(Debug, PartialEq)]
pub enum EVariable {
    Width,
    Height
}

/**
 * example:
 *
 * width <= 400 and height <= 400 and filename matches <foo/bar>
 */
pub fn parse(input: &str) -> Result<Expr, String> {
    let mut input = DataInput::new(input.as_bytes());
    exp().parse(&mut input).map_err(|it| s!(it))
}

/**
 * Expr ← Logic | Bool
 * Logic ← Bool LogicOp Bool
 * Bool ← Compare
 * BoolOp ← 'and' | 'or'
 * Compare ← Value CmpOp Value
 * CmpOp ← '<' | '<=' | '>' | '>=' | '='
 */

fn spaces() -> Parser<u8, ()> {
    one_of(b" \t\r\n").repeat(0..).discard()
}

fn number() -> Parser<u8, EValue> {
    let integer = one_of(b"0123456789").repeat(1..);
    let number = sym(b'-').opt() + integer;
    number.collect().convert(String::from_utf8).convert(|s|i64::from_str(&s)).map(EValue::Integer)
}

fn variable() -> Parser<u8, EValue> {
    let width: Parser<u8, EVariable> = seq(b"width").map(constant!(EVariable::Width));
    let height: Parser<u8, EVariable> = seq(b"height").map(constant!(EVariable::Height));

    (width | height).map(|it| EValue::Variable(it))
}

fn value() -> Parser<u8, EValue> {
     variable() | number()
}

fn comp_op() -> Parser<u8, ECompOp> {
    let eq = (seq(b"==") | seq(b"=")).map(constant!(ECompOp::Eq));
    let ne = seq(b"!=").map(constant!(ECompOp::Ne));
    let le = seq(b"<=").map(constant!(ECompOp::Le));
    let lt = seq(b"<").map(constant!(ECompOp::Lt));
    let ge = seq(b">=").map(constant!(ECompOp::Ge));
    let gt = seq(b">").map(constant!(ECompOp::Gt));

    eq | ne | lt | le | gt | ge
}

fn boolean() -> Parser<u8, Expr> {
    (value() + (spaces() * comp_op() - spaces()) + value()).map(|((l, op), r)| {
        Expr::Boolean(EBool::Compare(l, op, r))
    })
}

fn logic_op() -> Parser<u8, ELogicOp> {
    let and = seq(b"and").map(constant!(ELogicOp::And));
    let or = seq(b"or").map(constant!(ELogicOp::Or));

    and | or
}

fn logic() -> Parser<u8, Expr> {
    (boolean() + (spaces() * logic_op() - spaces()) + boolean()).map(|((l, op), r)| {
        Expr::Logic(Box::new(l), op, Box::new(r))
    })
}


fn exp() -> Parser<u8, Expr> {
    logic() | boolean()
}



#[cfg(test)]#[test]
fn test_parser() {
    use self::Expr::*;
    use self::EBool::*;
    use self::EValue::*;
    use self::ECompOp::*;
    use self::EVariable::*;
    use self::ELogicOp::*;

    assert_eq!(
        parse("1 < 2"),
        Ok(
            Boolean(
                Compare(
                    Integer(1),
                    Lt,
                    Integer(2)))));

    assert_eq!(
        parse("width < 200"),
        Ok(
            Boolean(
                Compare(
                    Variable(Width),
                    Lt,
                    Integer(200)))));

    assert_eq!(
        parse("width < 200 and height < 400"),
        Ok(
            Logic(
                Box::new(
                    Boolean(
                        Compare(
                            Variable(Width),
                            Lt,
                            Integer(200)))),
                And,
                Box::new(
                        Boolean(
                            Compare(
                                Variable(Height),
                                Lt,
                                Integer(400)))))));
}
