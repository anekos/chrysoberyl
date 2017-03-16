
use std::borrow::Cow;
use std::process::Command;
use std::thread::spawn;

use onig::Regex;
use shell_escape::escape;



pub fn call(async: bool, command_name: &str, arguments: &Vec<String>, info: Vec<(String, String)>) {
    let mut command = Command::new("bash");
    let mut command_line = command_name.to_owned();

    for argument in arguments {
        command_line.push(' ');
        if is_variable(argument) {
            command_line.push_str(&format!(r#""{}""#, argument));
        } else {
            let argument = Cow::from(argument.to_owned());
            command_line.push_str(&escape(argument).into_owned());
        }
    }

    command.arg("-c").arg(command_line);

    for (key, value) in info {
        command.env(format!("Chrysoberyl_{}", key).to_uppercase(), value);
    }

    let mut child = command.spawn().expect(&*format!("Failed to run: {}", command_name));
    if async {
        spawn(move || child.wait().unwrap());
    } else {
        child.wait().unwrap();
    }
}

fn is_variable(s: &str) -> bool {
    Regex::new(r#"^\$[a-zA-Z_][a-zA-Z_0-9]*$"#).unwrap().is_match(s)
}


#[cfg(test)]#[test]
fn test_is_variable() {
    assert!(!is_variable("hoge"));
    assert!(is_variable("$file"));
    assert!(is_variable("$CHRYSOBERYL_FILE"));
    assert!(!is_variable("$CHRYSOBERYL    FILE"));
}
