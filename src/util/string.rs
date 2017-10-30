
use std::fmt::Write;



pub fn join(xs: &[String], sep: char) -> String {
    let mut result = o!("");
    for x in xs {
        write!(result, "{}{}", x, sep).unwrap();
    }
    result.pop();
    result
}
