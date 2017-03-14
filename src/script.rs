
use std::process::Command;



pub fn call(command_name: &str, arguments: &Vec<String>, info: Vec<(String, String)>) {
    let mut command = Command::new("bash");
    command.arg("-c")
        .arg(command_name.clone())
        .args(arguments.as_slice())
        .env("PANTY_WINDOWID", "hoge");

    for (key, value) in info {
        command.env(format!("Chrysoberyl_{}", key).to_uppercase(), value);
    }


    let mut child = command.spawn().expect(&*format!("Failed to run: {}", command_name));
    child.wait().unwrap();
}
