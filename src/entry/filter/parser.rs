
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
    expr().parse(&mut input).map_err(|it| s!(it))
}

impl FromStr for Expr {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        parse(src)
    }
}


#[cfg_attr(feature = "cargo-clippy", allow(doc_markdown))]
/**
 * Expr ← Bool | If | Logic
 * Logic ← Bool LogicOp Expr
 * Bool ← Compare | BoolVariable
 * If ← 'if' Expr Expr Expr
 * BoolOp ← 'and' | 'or'
 * Compare ← Value CmpOp Value
 * CmpOp ← '<' | '<=' | '>' | '>=' | '=' | '=~'
 * Value ← Glob | Integer | Variable
 * Variable ← 'type' | 'width' | 'height' | 'path' | 'ext' | 'extension' | 'dimensions'
 * Glob ← '<' string '>'
 * BoolVariable ← 'animation'
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
        seq(name).map(move |_| EValue::Variable(var))
    }

    gen(b"type", Type) |
        gen(b"dimensions", Dimentions) |
        gen(b"dim", Dimentions) |
        gen(b"extension", Extension) |
        gen(b"ext", Extension) |
        gen(b"height", Height) |
        gen(b"name", Name) |
        gen(b"page", Page) |
        gen(b"path", Path) |
        gen(b"width", Width)
}

fn value() -> Parser<u8, EValue> {
     variable() | number() | glob()
}

fn comp_op() -> Parser<u8, ECompOp> {
    fn i(v: EICompOp) -> ECompOp {
        ECompOp::ForInt(v)
    }

    let eq = sym(b'=') * {
        let eq2 = sym(b'=').map(|_| i(EICompOp::Eq));
        let eq1 = empty().map(|_| i(EICompOp::Eq));
        let glob = sym(b'*').map(|_| ECompOp::GlobMatch(false));
        eq2 | eq1 | glob
    };

    let lt = sym(b'<') * {
        let le = sym(b'=').map(|_| i(EICompOp::Le));
        let lt = empty().map(|_| i(EICompOp::Lt));
        le | lt
    };

    let gt = sym(b'>') * {
        let ge = sym(b'=').map(|_| i(EICompOp::Ge));
        let gt = empty().map(|_| i(EICompOp::Gt));
        ge | gt
    };

    let not = sym(b'!') * {
        let ne = sym(b'=').map(|_| i(EICompOp::Ne));
        let glob_not = sym(b'*').map(|_| ECompOp::GlobMatch(true));
        ne | glob_not
    };

    eq | lt | gt | not
}

fn compare() -> Parser<u8, EBool> {
    (value() + (spaces() * comp_op() - spaces()) + value()).map(|((l, op), r)| {
        EBool::Compare(l, op, r)
    })
}

fn bool_variable() -> Parser<u8, EBool> {
    seq(b"animation").map(|_| EBool::Variable(EBVariable::Animation))
}

fn boolean() -> Parser<u8, Expr> {
    (bool_variable() | compare()).map(Expr::Boolean)
}

fn logic_op() -> Parser<u8, ELogicOp> {
    let and = seq(b"and").map(|_| (ELogicOp::And));
    let or = seq(b"or").map(|_| (ELogicOp::Or));

    and | or
}

fn logic() -> Parser<u8, Expr> {
    (boolean() + (spaces() * logic_op() - spaces()) + call(expr_item)).map(|((l, op), r)| {
        Expr::Logic(Box::new(l), op, Box::new(r))
    })
}

fn glob() -> Parser<u8, EValue> {
    (sym(b'<') * list(call(glob_entry), sym(b',')) - sym(b'>')).map(|entries| {
        EValue::Glob(entries.into_iter().map(|(m, src)| (m, src)).collect())
    })
}

fn glob_entry() -> Parser<u8, (globset::GlobMatcher, String)> {
    none_of(b",>").repeat(0..).convert(String::from_utf8).convert(|src| {
        globset::Glob::new(&src).map(|it| {
            (it.compile_matcher(), src)
        })
    })
}

fn if_() -> Parser<u8, Expr> {
    let p = seq(b"if") * spaces() * (call(expr_item) + (spaces() * call(expr_item)) + (spaces() * call(expr_item)));
    p.map(|((cond, true_clause), false_clause)| Expr::If(Box::new(cond), Box::new(true_clause), Box::new(false_clause)))
}

fn expr_item() -> Parser<u8, Expr> {
    call(logic) | boolean() | call(if_)
}


fn expr() -> Parser<u8, Expr> {
    spaces() * expr_item() - spaces()
}



#[cfg(test)]#[test]
fn test_parser() {
    use session::write_filter;

    fn assert_parse(src: &str) {
        assert_eq!(
            parse(src).map(|it| {
                let mut parsed = o!("");
                write_filter(&Some(it), &mut parsed);
                parsed
            }),
            Ok(format!("@filter {}\n", src)))
    }

    fn assert_parse2(src: &str, expect: &str) {
        assert_eq!(
            parse(src).map(|it| {
                let mut parsed = o!("");
                write_filter(&Some(it), &mut parsed);
                parsed
            }),
            Ok(format!("@filter {}\n", expect)))
    }

    assert_parse("1 < 2");
    assert_parse("width < 200");
    assert_parse("width < 200 and height < 400");
    assert_parse("width < 200 and height < 400");
    assert_parse("width < 200 and height < 400 and extension == <jpg>");
    assert_parse("if path == <*.google.com*> width < 200 height < 400");

    assert_parse("dimensions == 12345");
    assert_parse2("dim == 12345", "dimensions == 12345");

    assert_parse("extension == <hoge>");
    assert_parse2("ext == <hoge>", "extension == <hoge>");
}
