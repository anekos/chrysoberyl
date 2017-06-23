
use entry::filter::expression::*;


pub fn write(expr: &Expr, out: &mut String) {
    use self::Expr::*;

    match *expr {
        Logic(ref l, ref op, ref r) =>
            write_logic(l, op, r, out),
        Boolean(ref v) =>
            write_bool(v, out),
    }
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

    match *b {
        Compare(ref l, ref op, ref r) => {
            write_value(l, out);
            write_space(out);
            match *op {
                Eq => sprint!(out, "=="),
                Lt => sprint!(out, "<"),
                Le => sprint!(out, "<="),
                Gt => sprint!(out, ">"),
                Ge => sprint!(out, ">="),
                Ne => sprint!(out, "!="),
            }
            write_space(out);
            write_value(r, out);
        }
    }
}

fn write_value(v: &EValue, out: &mut String) {
    use self::EValue::*;
    use self::EVariable::*;

    match *v {
        Integer(ref v) => sprint!(out, "{}", v),
        Variable(ref v) => match *v {
            Width => sprint!(out, "width"),
            Height => sprint!(out, "height"),
        }
    }
}

fn write_space(out: &mut String) {
    sprint!(out, " ");
}
