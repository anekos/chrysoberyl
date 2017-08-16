
use globset::GlobMatcher;

use entry::{Entry, EntryContent};
use entry::info::{EntryInfo, LazyEntryInfo};
use resolution;

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
        If(ref cond, ref true_clause, ref false_clause) =>
            eval_if(info, content, cond, true_clause, false_clause),
        When(reverse, ref cond, ref clause) =>
            eval_when(info, content, reverse, cond, clause),
        Boolean(ref b) =>
            eval_bool(info, content, b),
        Logic(ref l, ref op, ref r) =>
            eval_logic(info, content, l, op, r),
        Not(ref expr) =>
            !eval(info, content, expr),
    }
}

fn eval_if(info: &mut EntryInfo, content: &EntryContent, cond: &Expr, true_clause: &Expr, false_clause: &Expr) -> bool {
    if eval(info, content, cond) {
        eval(info, content, true_clause)
    } else {
        eval(info, content, false_clause)
    }
}

fn eval_when(info: &mut EntryInfo, content: &EntryContent, reverse: bool, cond: &Expr, clause: &Expr) -> bool {
    if reverse ^ eval(info, content, cond) {
        eval(info, content, clause)
    } else {
        true
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
        },
        Resolution(w, h) =>
            return resolution_match(info.lazy(content), w, h),
        True =>
            return true,
        False =>
            return false,
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
        Page => Some(info.strict.page),
        FileSize => Some(info.lazy(content).file_size as i64),
        Type | Path | Name | Extension => None,
    }
}

fn eval_variable_as_s(info: &EntryInfo, v: &EVariable) -> Option<String> {
    use self::EVariable::*;

    match *v {
        Path => Some(info.strict.path.clone()),
        Extension => info.strict.extension.clone(),
        Type => Some(o!(info.strict.entry_type)),
        Name => Some(info.strict.name.clone()),
        Page | Dimentions | Width | Height | FileSize => None,
    }
}

fn resolution_match(info: &LazyEntryInfo, w: i64, h: i64) -> bool {
    if_let_some!(dim = info.dimensions, false);
    dim.width as i64 == w && dim.height as i64 == h
}
