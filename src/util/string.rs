
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

pub fn remove_linebreaks(src: &str) -> String {
    src.replace('\n', "").replace('\r', "")
}

pub fn substr(src: &str, begin: usize, end: usize) -> &str {
    if end <= begin {
        return "";
    }

    let mut ci = src.char_indices();
    let left = ci.nth(begin).map(|it| it.0).unwrap_or(0);
    let right = ci.nth(end - begin).map(|it| it.0).unwrap_or_else(|| src.len());
    &src[left..right]
}
