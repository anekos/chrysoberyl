
use std::io::{Write, stderr};
use std::fmt::Display;



pub fn error<T: Display>(message: T) {
    writeln!(&mut stderr(), "Error\t{}", message).unwrap();
}


macro_rules! puts {
    ( $name:expr $(,$arg:expr)* ) => {
        {
            print!("{}", $name);
            $( print!("\t{}", $arg); )*
            println!("");
        }
    }
}
