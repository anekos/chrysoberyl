
use std::fmt::Write;



pub fn join(xs: &[String], sep: char) -> String {
    let mut result = o!("");
    for x in xs {
        write!(result, "{}{}", x, sep).unwrap();
    }
    result.pop();
    result
}


pub fn prefixed_lines(prefix: &str, source: &str) -> String {
    let mut result = o!("");
    for line in source.lines() {
        result.push_str(prefix);
        result.push_str(line);
        result.push_str("\n");
    }
    result
}
