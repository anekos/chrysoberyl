
use std::path::Path;

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
            eval_logic(info, l, op, r)
    }
}

fn eval_bool(info: &EntryInfo, b: &EBool) -> bool {
    use self::EBool::*;
    use self::ECompOp::*;

    match *b {
        Compare(ref l, ref op, ref r) => {
            if let (Some(l), Some(r)) = (eval_value(info, l), eval_value(info, r)) {
                match *op {
                    Eq => l == r,
                    Lt => l < r,
                    Le => l <= r,
                    Gt => l > r,
                    Ge => l >= r,
                    Ne => l != r,
                }
            } else {
                true // XXX
            }
        }
    }
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

fn eval_value(info: &EntryInfo, v: &EValue) -> Option<i64> {
    use self::EValue::*;

    match *v {
        Integer(v) =>
            Some(v),
        Variable(ref v) =>
            eval_variable(info, v)
    }
}

fn eval_variable(info: &EntryInfo, v: &EVariable) -> Option<i64> {
    use self::EVariable::*;

    match *v {
        Width => info.size.map(|it| it.width as i64),
        Height => info.size.map(|it| it.height as i64),
    }
}

fn match_extensions(path: &str, extensions: &[String]) -> bool {
    if_let_some!(ext = Path::new(path).extension(), true);
    let ext = ext.to_str().unwrap().to_lowercase();

    for extension in extensions {
        if &ext == extension {
            return true
        }
    }

    false
}
