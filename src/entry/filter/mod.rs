
use globset::GlobMatcher;

use app::info::AppInfo;
use entry::info::EntryInfo;
use entry::{Entry, EntryContent};
use resolution;
use size::Size;

pub mod expression;
pub mod parser;
pub mod writer;

use self::expression::*;



struct Info<'a> {
    app: &'a AppInfo,
    entry: &'a EntryInfo,
}

impl Expr {
    pub fn evaluate(&self, entry: &Entry, app_info: &AppInfo) -> bool {
        let info = Info { app: app_info, entry: &entry.info };
        eval(&info, &entry.content, self)
    }
}


fn eval(info: &Info, content: &EntryContent, expr: &Expr) -> bool {
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

fn eval_if(info: &Info, content: &EntryContent, cond: &Expr, true_clause: &Expr, false_clause: &Expr) -> bool {
    if eval(info, content, cond) {
        eval(info, content, true_clause)
    } else {
        eval(info, content, false_clause)
    }
}

fn eval_when(info: &Info, content: &EntryContent, reverse: bool, cond: &Expr, clause: &Expr) -> bool {
    if reverse ^ eval(info, content, cond) {
        eval(info, content, clause)
    } else {
        true
    }
}

fn eval_bool(info: &Info, content: &EntryContent, b: &EBool) -> bool {
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
                    if let (Some(ref l), Some(ref rs)) = (eval_value_as_s(info, content, l), eval_value_as_g(r)) {
                        return rs.iter().any(|r| r.is_match(l)) ^ inverse;
                    }
                }
            }
        },
        Variable(ref name) => {
            return match *name {
                Active => info.app.active,
                Animation => info.entry.lazy(content, |it| it.is_animated)
            }
        },
        Resolution(w, h) =>
            return resolution_match(info.entry.lazy(content, |it| it.dimensions), w, h),
        True =>
            return true,
        False =>
            return false,
    }

    true
}

fn eval_logic(info: &Info, content: &EntryContent, l: &Expr, op: &ELogicOp, r: &Expr) -> bool {
    use self::ELogicOp::*;

    let l = eval(info, content, l);
    let r = eval(info, content, r);

    match *op {
        And => l && r,
        Or => l || r,
    }
}

fn eval_value_as_i(info: &Info, content: &EntryContent, v: &EValue) -> Option<i64> {
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

fn eval_value_as_s(info: &Info, content: &EntryContent, v: &EValue) -> Option<String> {
    use self::EValue::*;

    match *v {
        Integer(_) | Glob(_) =>
            None,
        Variable(ref v) =>
            eval_variable_as_s(info, content, v)
    }
}

fn eval_variable(info: &Info, content: &EntryContent, v: &EVariable) -> Option<i64> {
    use self::EVariable::*;

    match *v {
        ArchivePage => Some(info.entry.strict.archive_page),
        CurrentPage => info.app.current_page.map(|it| it as i64),
        Width => info.entry.lazy(content, |it| it.dimensions).map(|it| i64!(it.width)),
        Height => info.entry.lazy(content, |it| it.dimensions).map(|it| i64!(it.height)),
        Dimentions => info.entry.lazy(content, |it| it.dimensions).map(|it| i64!(it.dimensions())),
        Pages => Some(info.app.pages as i64),
        RealPages => Some(info.app.real_pages as i64),
        FileSize => info.entry.lazy(content, |it| it.file_size).map(|it| it as i64),
        AspectRatio | Type | Path | Name | Extension => None,
    }
}

fn eval_variable_as_s(info: &Info, content: &EntryContent, v: &EVariable) -> Option<String> {
    use self::EVariable::*;

    match *v {
        AspectRatio => info.entry.lazy(content, |it| it.dimensions).map(|it| {
            let (w, h) = it.ratio();
            format!("{}:{}", w, h)
        }),
        Path => Some(info.entry.strict.path.clone()),
        Extension => info.entry.strict.extension.clone(),
        Type => Some(o!(info.entry.strict.entry_type)),
        Name => Some(info.entry.strict.name.clone()),
        ArchivePage | CurrentPage | Pages | RealPages | Dimentions | Width | Height | FileSize => None,
    }
}

fn resolution_match(dims: Option<Size>, w: i64, h: i64) -> bool {
    if_let_some!(dim = dims, false);
    i64!(dim.width) == w && i64!(dim.height) == h
}
