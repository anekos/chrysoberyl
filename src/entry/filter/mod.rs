
use globset::GlobMatcher;

use entry::{Entry, EntryInfo};

mod info;
pub mod expression;
pub mod parser;
pub mod writer;

use self::expression::*;



impl Expr {
    pub fn evaluate(&self, entry: &mut Entry) -> bool {
        let info = info::get_info(entry);
        eval(info, self)
    }
}


fn eval(info: &EntryInfo, expr: &Expr) -> bool {
    use self::Expr::*;

    match *expr {
        Boolean(ref b) =>
            eval_bool(info, b),
        Logic(ref l, ref op, ref r) =>
            eval_logic(info, l, op, r),
    }
}

fn eval_bool(info: &EntryInfo, b: &EBool) -> bool {
    use self::EBool::*;
    use self::ECompOp::*;
    use self::EICompOp::*;

    match *b {
        Compare(ref l, ref op, ref r) => {
            match *op {
                ForInt(ref op) => {
                    if let (Some(l), Some(r)) = (eval_value_as_i(info, l), eval_value_as_i(info, r)) {
                        return match *op {
                            Eq => l == r,
                            Lt => l < r,
                            Le => l <= r,
                            Gt => l > r,
                            Ge => l >= r,
                            Ne => l != r,
                        };
                    } else if *op == Eq || *op == Ne {
                        let b = Compare(l.clone(), GlobMatch(*op == Ne), r.clone());
                        return eval_bool(info, &b);
                    }
                }
                GlobMatch(inverse) => {
                    if let (Some(ref l), Some(ref rs)) = (eval_value_as_s(info, l), eval_value_as_g(r)) {
                        return rs.iter().any(|r| r.is_match(l)) ^ inverse;
                    }
                }
            }
        }
    }

    true
}

fn eval_logic(info: &EntryInfo, l: &Expr, op: &ELogicOp, r: &Expr) -> bool {
    use self::ELogicOp::*;

    let l = eval(info, l);
    let r = eval(info, r);

    match *op {
        And => l && r,
        Or => l || r,
    }
}

fn eval_value_as_i(info: &EntryInfo, v: &EValue) -> Option<i64> {
    use self::EValue::*;

    match *v {
        Integer(v) =>
            Some(v),
        Variable(ref v) =>
            eval_variable(info, v),
        Glob(_) =>
            None,
    }
}

fn eval_value_as_g(v: &EValue) -> Option<Vec<GlobMatcher>> {
    use self::EValue::*;

    match *v {
        Integer(_) | Variable(_) =>
            None,
        Glob(ref globs) =>
            Some(globs.iter().map(|it| it.0.clone()).collect()),
    }
}

fn eval_value_as_s(info: &EntryInfo, v: &EValue) -> Option<String> {
    use self::EValue::*;

    match *v {
        Integer(_) | Glob(_) =>
            None,
        Variable(ref v) =>
            eval_variable_as_s(info, v)
    }
}

fn eval_variable(info: &EntryInfo, v: &EVariable) -> Option<i64> {
    use self::EVariable::*;

    match *v {
        Width => info.dimensions.map(|it| it.width as i64),
        Height => info.dimensions.map(|it| it.height as i64),
        Dimentions => info.dimensions.map(|it| it.dimensions() as i64),
        _ => None,
    }
}

fn eval_variable_as_s(info: &EntryInfo, v: &EVariable) -> Option<String> {
    use self::EVariable::*;

    match *v {
        Path => Some(info.path.clone()),
        Extension => info.extension.clone(),
        Type => Some(o!(info.entry_type.clone())),
        _ => None,
    }
}
