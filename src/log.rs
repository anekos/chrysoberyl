
use std::io::{Write, stderr};
use std::fmt::Display;



pub fn error<T: Display>(message: T) {
    writeln!(&mut stderr(), "Error\t{}", message).unwrap();
}


pub fn puts1<T: Display>(action_name: &str, arg1: T) {
    println!("{}\t{}", action_name, arg1);
}

pub fn puts2<T1: Display, T2: Display>(action_name: &str, arg1: T1, arg2: T2) {
    println!("{}\t{}\t{}", action_name, arg1, arg2);
}
