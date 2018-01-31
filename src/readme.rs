
pub const README: &str = include_str!("../README.md");



pub fn body() -> Vec<String> {
    let mut result = vec![];
    let mut phase = 0;

    for line in README.lines() {
        match phase {
            0 if line == "# Command line" => phase = 1,
            1 if line == "```" => phase = 2,
            2 if line == "```" => phase = 3,
            2 => result.push(format!("  {}", line)),
            3 => result.push(o!(line)),
            _ => (),
        }
    }

    result
}
