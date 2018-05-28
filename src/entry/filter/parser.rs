
use std::str::FromStr;

use globset;
use pom::parser::*;
use pom::{Parser, TextInput};

use entry::filter::expression::*;
use entry::filter::resolution;
use util::pom::from_vec_char;



/**
 * example:
 *
 * width <= 400 and height <= 400 and filename matches <foo/bar>
 */
pub fn parse(input: &str) -> Result<Expr, String> {
    let mut input = TextInput::new(input);
    expr().parse(&mut input).map_err(|it| s!(it))
}

impl FromStr for Expr {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        parse(src)
    }
}


fn spaces() -> Parser<char, ()> {
    one_of(" \t\r\n").repeat(0..).discard()
}

fn number() -> Parser<char, EValue> {
    let integer = one_of("0123456789").repeat(1..);
    let number = sym('-').opt() + integer;
    let suffix = (one_of("KMGTP") + sym('i').opt()).opt().map(|suffix| {
        if let Some((c, i)) = suffix {
            let p = match c {
                'K' => 1,
                'M' => 2,
                'G' => 3,
                'T' => 4,
                'P' => 5,
                _ => panic!("Unexpected char for integer suffix: {}", c)
            };
            let base: i64 = if i.is_some() { 1024 } else { 1000 };
            base.pow(p)
        } else {
            1
        }
    });
    let number = number.collect().map(from_vec_char).convert(|s|i64::from_str(&s));
    (number + suffix).map(|(n, s)| EValue::Integer(n * s))
}

fn variable() -> Parser<char, EValue> {
    use self::EVariable::*;

    fn gen(name: &'static str, var: EVariable) -> Parser<char, EValue> {
        seq(name).map(move |_| EValue::Variable(var))
    }

    gen("type", Type) |
        gen("archive-page", ArchivePage) |
        gen("current-page", CurrentPage) |
        gen("dimensions", Dimentions) |
        gen("dim", Dimentions) |
        gen("extension", Extension) |
        gen("ext", Extension) |
        gen("height", Height) |
        gen("name", Name) |
        gen("pages", Pages) |
        gen("path", Path) |
        gen("real-pages", RealPages) |
        gen("width", Width) |
        gen("filesize", FileSize) |
        gen("ratio", AspectRatio)
}

fn value() -> Parser<char, EValue> {
     variable() | number() | glob()
}

fn comp_op() -> Parser<char, ECompOp> {
    fn i(v: EICompOp) -> ECompOp {
        ECompOp::ForInt(v)
    }

    let eq = sym('=') * {
        let eq2 = sym('=').map(|_| i(EICompOp::Eq));
        let glob = sym('*').map(|_| ECompOp::GlobMatch(false));
        let eq1 = empty().map(|_| i(EICompOp::Eq));
        eq2 | glob | eq1
    };

    let lt = sym('<') * {
        let le = sym('=').map(|_| i(EICompOp::Le));
        let lt = empty().map(|_| i(EICompOp::Lt));
        le | lt
    };

    let gt = sym('>') * {
        let ge = sym('=').map(|_| i(EICompOp::Ge));
        let gt = empty().map(|_| i(EICompOp::Gt));
        ge | gt
    };

    let not = sym('!') * {
        let ne = sym('=').map(|_| i(EICompOp::Ne));
        let glob_not = sym('*').map(|_| ECompOp::GlobMatch(true));
        ne | glob_not
    };

    eq | lt | gt | not
}

fn compare() -> Parser<char, EBool> {
    (value() + (spaces() * comp_op() - spaces()) + value()).map(|((l, op), r)| {
        EBool::Compare(l, op, r)
    })
}

fn bool_variable() -> Parser<char, EBool> {
    use self::EBVariable::*;

    fn gen(name: &'static str, var: EBVariable) -> Parser<char, EBool> {
        seq(name).map(move |_| EBool::Variable(var))
    }

    gen("active", Active) | gen("animation", Animation)
}

fn lit_true() -> Parser<char, EBool> {
    seq("true").map(|_| EBool::True)
}

fn lit_false() -> Parser<char, EBool> {
    seq("false").map(|_| EBool::False)
}

fn boolean() -> Parser<char, Expr> {
    (bool_variable() | compare() | resolution() | lit_true() | lit_false()).map(Expr::Boolean)
}

fn logic_op() -> Parser<char, ELogicOp> {
    let and = seq("and").map(|_| (ELogicOp::And));
    let or = seq("or").map(|_| (ELogicOp::Or));

    and | or
}

fn logic() -> Parser<char, Expr> {
    (boolean() + (spaces() * logic_op() - spaces()) + call(expr_item)).map(|((l, op), r)| {
        Expr::Logic(Box::new(l), op, Box::new(r))
    })
}

fn glob() -> Parser<char, EValue> {
    (sym('<') * list(call(glob_entry), sym(',')) - sym('>')).map(|entries| {
        EValue::Glob(entries.into_iter().map(|(m, src)| (m, src)).collect())
    })
}

fn glob_entry() -> Parser<char, (globset::GlobMatcher, String)> {
    none_of(",>").repeat(0..).map(from_vec_char).convert(|src| {
        globset::Glob::new(&src).map(|it| {
            (it.compile_matcher(), src)
        })
    })
}

fn resolution() -> Parser<char, EBool> {
    let integer = || one_of("0123456789").repeat(1..).map(from_vec_char).convert(|s|i64::from_str(&s));
    let ixi = integer() + (sym('x') * integer());
    let name = none_of(" ").repeat(1..).convert(resolution::from_vec);

    (sym('?') * (name | ixi)).map(|(w, h)| EBool::Resolution(w, h))
}

fn when() -> Parser<char, Expr> {
    let p = (seq("when") | seq("unless")) - spaces() + (call(expr_item) + (spaces() * call(expr_item)));
    p.map(|(when_unless, (cond, clause))| {
        let when_unless = from_vec_char(when_unless);
        Expr::When(when_unless == "unless", Box::new(cond), Box::new(clause))
    })
}

fn if_() -> Parser<char, Expr> {
    let p = seq("if") * spaces() * (call(expr_item) + (spaces() * call(expr_item)) + (spaces() * call(expr_item)));
    p.map(|((cond, true_clause), false_clause)| Expr::If(Box::new(cond), Box::new(true_clause), Box::new(false_clause)))
}

fn block_paren() -> Parser<char, Expr> {
    sym('(') * spaces() * call(expr_item) - spaces() - sym(')')
}

fn block_curly() -> Parser<char, Expr> {
    sym('{') * spaces() * call(expr_item) - spaces() - sym('}')
}

fn block() -> Parser<char, Expr> {
    block_paren() | block_curly()
}

fn not() -> Parser<char, Expr> {
    seq("not") * spaces() * expr_item().map(|expr| Expr::Not(Box::new(expr)))
}

fn expr_item() -> Parser<char, Expr> {
    block() | call(logic) | boolean() | call(if_) | call(when) | call(not)
}


fn expr() -> Parser<char, Expr> {
    spaces() * expr_item() - spaces()
}



#[cfg(test)]#[test]
fn test_parser() {
    use session::write_filter;
    use util::shell::escape;

    fn assert_parse(src: &str) {
        assert_eq!(
            parse(src).map(|it| {
                let mut parsed = o!("");
                write_filter(&Some(it), "", &mut parsed);
                parsed
            }),
            Ok(format!("@filter {}\n", escape(src))))
    }

    fn assert_parse2(src: &str, expect: &str) {
        assert_eq!(
            parse(src).map(|it| {
                let mut parsed = o!("");
                write_filter(&Some(it), "", &mut parsed);
                parsed
            }),
            Ok(format!("@filter {}\n", escape(expect))))
    }

    assert_parse("1 < 2");
    assert_parse("width < 200");
    assert_parse("width < 200 and height < 400");
    assert_parse("width < 200 and height < 400");
    assert_parse("width < 200 and height < 400 and extension == <jpg>");

    assert_parse("when path == <google> width < 200");
    assert_parse("unless path == <google> width < 200");
    assert_parse("if path == <*.google.com*> width < 200 height < 400");

    assert_parse2("if (path == <google>) (width < 200) (height < 400)", "if path == <google> width < 200 height < 400");
    assert_parse2("if (path == <google>) {width < 200} {height < 400}", "if path == <google> width < 200 height < 400");

    assert_parse("dimensions == 12345");
    assert_parse2("dim == 12345", "dimensions == 12345");
    assert_parse("extension == <hoge>");
    assert_parse2("ext == <hoge>", "extension == <hoge>");

    assert_parse("path =* <hoge>");
    assert_parse("path !* <hoge>");

    assert_parse("width < 2K");
    assert_parse("width < 2Ki");
    assert_parse2("width < 2000", "width < 2K");
    assert_parse2("width < 2048", "width < 2Ki");
    assert_parse2("width < -2048", "width < -2Ki");

    assert_parse("not (width < 200 and height < 400)");

    for c in "KMGTP".chars() {
        assert_parse(&format!("width < 9{}", c));
        assert_parse(&format!("width < 9{}i", c));
    }

    assert_parse("?100x200");
    assert_parse("?VGA");
    assert_parse2("?640x480", "?VGA");
}
