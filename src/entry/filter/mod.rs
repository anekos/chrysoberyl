
use globset::GlobMatcher;

use entry::{Entry, EntryContent};
use entry::info::EntryInfo;

pub mod expression;
pub mod parser;
pub mod writer;

use self::expression::*;



impl Expr {
    pub fn evaluate(&self, entry: &mut Entry) -> bool {
        eval(&mut entry.info, &entry.content, self)
    }
}


fn eval(info: &mut EntryInfo, content: &EntryContent, expr: &Expr) -> bool {
    use self::Expr::*;

    match *expr {
        Boolean(ref b) =>
            eval_bool(info, content, b),
        Logic(ref l, ref op, ref r) =>
            eval_logic(info, content, l, op, r),
    }
}

fn eval_bool(info: &mut EntryInfo, content: &EntryContent, b: &EBool) -> bool {
    use self::EBool::*;
    use self::ECompOp::*;
    use self::EICompOp::*;
    use self::EBVariable::*;

    match *b {
        Compare(ref l, ref op, ref r) => {
            match *op {
                ForInt(ref op) => {
                    if let (Some(l), Some(r)) = (eval_value_as_i(info, content, l), eval_value_as_i(info, content, r)) {
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
                        return eval_bool(info, content, &b);
                    }
                }
                GlobMatch(inverse) => {
                    if let (Some(ref l), Some(ref rs)) = (eval_value_as_s(info, l), eval_value_as_g(r)) {
                        return rs.iter().any(|r| r.is_match(l)) ^ inverse;
                    }
                }
            }
        },
        Variable(ref name) => {
            return match *name {
                Animation => info.lazy(content).is_animated
            }
        }
    }

    true
}

fn eval_logic(info: &mut EntryInfo, content: &EntryContent, l: &Expr, op: &ELogicOp, r: &Expr) -> bool {
    use self::ELogicOp::*;

    let l = eval(info, content, l);
    let r = eval(info, content, r);

    match *op {
        And => l && r,
        Or => l || r,
    }
}

fn eval_value_as_i(info: &mut EntryInfo, content: &EntryContent, v: &EValue) -> Option<i64> {
    use self::EValue::*;

    match *v {
        Integer(v) =>
            Some(v),
        Variable(ref v) =>
            eval_variable(info, content, v),
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

fn eval_variable(info: &mut EntryInfo, content: &EntryContent, v: &EVariable) -> Option<i64> {
    use self::EVariable::*;

    match *v {
        Width => info.lazy(content).dimensions.map(|it| it.width as i64),
        Height => info.lazy(content).dimensions.map(|it| it.height as i64),
        Dimentions => info.lazy(content).dimensions.map(|it| it.dimensions() as i64),
        _ => None,
    }
}

fn eval_variable_as_s(info: &EntryInfo, v: &EVariable) -> Option<String> {
    use self::EVariable::*;

    match *v {
        Path => Some(info.strict.path.clone()),
        Extension => info.strict.extension.clone(),
        Type => Some(o!(info.strict.entry_type)),
        _ => None,
    }
}
