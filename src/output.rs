
use shell_escape::escape;
use std::borrow::Cow;


pub fn puts(data: &Vec<(String, String)>) {
    for (index, pair) in data.iter().enumerate() {
        let (ref key, ref value) = *pair;
        let value = Cow::from(format!("{}", value));
        if index == 0 {
            print!(":;");
        }
        print!(" {}={}", key, escape(value));
    }
    println!("");
}


macro_rules! puts {
    ( $($name:expr => $value:expr),* ) => {
        {
            use output;
            output::puts(&vec![
                $( ($name.to_owned(), format!("{}", $value)) ),*
            ])
        }
    }
}

macro_rules! puts_inner {
    ( $name:expr => $value:expr $(,$tname:expr => $tvalue:expr)* ) => {
        {
            let value = Cow::from(format!("{}", $value));
            print!(":; {}={}", $name, escape(value));
            $(
                let value = Cow::from(format!("{}", $tvalue));
                print!(" {}={}", $tname, escape(value));
            )*
        }
    };
}

macro_rules! puts_event {
    ( $event:expr  $(,$name:expr => $value:expr)* ) => {
        puts!("event" => $event $(, $name => $value)*)
    }
}

macro_rules! puts_error {
    ( $($name:expr => $value:expr),* ) => {
        puts!("event" => "error" $(, $name => $value)*)
    }
}
