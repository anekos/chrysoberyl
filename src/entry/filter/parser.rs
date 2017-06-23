
use std::str::FromStr;

use globset;
use pom::parser::*;
use pom::{Parser, DataInput};

use entry::filter::expression::*;



/**
 * example:
 *
 * width <= 400 and height <= 400 and filename matches <foo/bar>
 */
pub fn parse(input: &str) -> Result<Expr, String> {
    let mut input = DataInput::new(input.as_bytes());
    exp().parse(&mut input).map_err(|it| s!(it))
}

impl FromStr for Expr {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        parse(src)
    }
}


/**
 * Expr ← Logic | Bool
 * Logic ← Bool LogicOp Bool
 * Bool ← Compare
 * BoolOp ← 'and' | 'or'
 * Compare ← Value CmpOp Value
 * CmpOp ← '<' | '<=' | '>' | '>=' | '=' | '=~'
 * Value ← Glob | Integer | Variable
 * Variable ← 'width' | 'height'
 * Glob ← '<' string '>'
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
    use self::EVariable::*;

    fn gen(name: &'static [u8], var: EVariable) -> Parser<u8, EValue> {
        seq(name).map(constant!(EValue::Variable(var)))
    }

    gen(b"width", Width) | gen(b"height", Height) | gen(b"path", Path)
}

fn value() -> Parser<u8, EValue> {
     variable() | number() | glob()
}

fn comp_op() -> Parser<u8, ECompOp> {
    let eq = (seq(b"==") | seq(b"=")).map(constant!(EICompOp::Eq));
    let ne = seq(b"!=").map(constant!(EICompOp::Ne));
    let le = seq(b"<=").map(constant!(EICompOp::Le));
    let lt = seq(b"<").map(constant!(EICompOp::Lt));
    let ge = seq(b">=").map(constant!(EICompOp::Ge));
    let gt = seq(b">").map(constant!(EICompOp::Gt));
    let match_ = seq(b"=~").map(constant!(ECompOp::Match));

    let i = (eq | ne | lt | le | gt | ge).map(ECompOp::ForInt);
    match_ | i
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

fn glob() -> Parser<u8, EValue> {
    (sym(b'<') * none_of(b">").repeat(0..) - sym(b'>')).convert(String::from_utf8).convert(|src| {
        globset::Glob::new(&src).map(|it| {
            EValue::Glob(it.compile_matcher(), src)
        })
    })
}


fn exp() -> Parser<u8, Expr> {
    spaces() * (logic() | boolean()) - spaces()
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
