
use entry::filter::expression::*;
use entry::filter::resolution;



const SUFFIXES: &'static str = "KMGTP";


pub fn write(expr: &Expr, out: &mut String) {
    use self::Expr::*;

    match *expr {
        If(ref cond, ref true_clause, ref false_clause) =>
            write_if(cond, true_clause, false_clause, out),
        When(reverse, ref cond, ref clause) =>
            write_when(reverse, cond, clause, out),
        Logic(ref l, ref op, ref r) =>
            write_logic(l, op, r, out),
        Boolean(ref v) =>
            write_bool(v, out),
    }
}

fn write_if(cond: &Expr, true_clause: &Expr, false_clause: &Expr, out: &mut String) {
    sprint!(out, "if");
    write_space(out);
    write(cond, out);
    write_space(out);
    write(true_clause, out);
    write_space(out);
    write(false_clause, out);
}

fn write_when(reverse: bool, cond: &Expr, clause: &Expr, out: &mut String) {
    if reverse {
        sprint!(out, "unless");
    } else {
        sprint!(out, "when");
    }
    write_space(out);
    write(cond, out);
    write_space(out);
    write(clause, out);
}

fn write_logic(l: &Expr, op: &ELogicOp, r: &Expr, out: &mut String) {
    use self::ELogicOp::*;

    write(l, out);
    write_space(out);
    match *op {
        And => sprint!(out, "and"),
        Or => sprint!(out, "or"),
    }
    write_space(out);
    write(r, out);
}

fn write_bool(b: &EBool, out: &mut String) {
    use self::EBool::*;
    use self::ECompOp::*;
    use self::EICompOp::*;
    use self::EBVariable::*;

    match *b {
        Compare(ref l, ref op, ref r) => {
            write_value(l, out);
            write_space(out);
            match *op {
                ForInt(ref op) => {
                    match *op {
                        Eq => sprint!(out, "=="),
                        Lt => sprint!(out, "<"),
                        Le => sprint!(out, "<="),
                        Gt => sprint!(out, ">"),
                        Ge => sprint!(out, ">="),
                        Ne => sprint!(out, "!="),
                    }
                },
                GlobMatch(false) => sprint!(out, "=*"),
                GlobMatch(true) => sprint!(out, "!*"),
            }
            write_space(out);
            write_value(r, out);
        }
        Variable(ref name) => {
            match *name {
                Animation => sprint!(out, "animation")
            }
        },
        Resolution(w, h) =>
            sprint!(out, "?{}", resolution::to(w, h)),
        True =>
            sprint!(out, "true"),
        False =>
            sprint!(out, "false"),
    }
}

fn write_value(v: &EValue, out: &mut String) {
    use self::EValue::*;
    use self::EVariable::*;

    match *v {
        Integer(v) => sprint!(out, "{}", suffixed_integer(v)),
        Variable(ref v) => match *v {
            Dimentions => sprint!(out, "dimensions"),
            Extension => sprint!(out, "extension"),
            Height => sprint!(out, "height"),
            Page => sprint!(out, "page"),
            Path => sprint!(out, "path"),
            Type => sprint!(out, "type"),
            Width => sprint!(out, "width"),
            Name => sprint!(out, "name"),
            FileSize => sprint!(out, "filesize"),
        },
        Glob(ref rs) => {
            sprint!(out, "<");
            for (index, r) in rs.iter().enumerate() {
                if 0 < index {
                    sprint!(out, ",");
                }
                sprint!(out, &r.1);
            }
            sprint!(out, ">");
        }
    }
}

fn write_space(out: &mut String) {
    sprint!(out, " ");
}

fn suffixed_integer(n: i64) -> String {
    let f = |base: i64| -> (i64, usize) {
        let mut p = 0;
        let mut m = n;
        for _ in SUFFIXES.chars() {
            if m % base == 0 {
                m /= base;
                p += 1;
            } else {
                break;
            }
        }

        (m, p)
    };

    let (mi, pi) = f(1024);
    let (m, p) = f(1000);

    if pi == 0 && p == 0 {
        format!("{}", n)
    } else if p < pi {
        format!("{}{}i", mi, SUFFIXES.chars().nth(pi - 1).unwrap())
    } else {
        format!("{}{}", m, SUFFIXES.chars().nth(p - 1).unwrap())
    }
}
