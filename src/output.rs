


macro_rules! puts {
    ( $($name:expr => $value:expr),* ) => {
        {
            use shell_escape::escape;
            use std::borrow::Cow;
            puts_inner!($($name => $value),*);
            println!("");
        }
    }
}

macro_rules! puts_inner {
    ( $name:expr => $value:expr $(,$tname:expr => $tvalue:expr)* ) => {
        {
            let value = Cow::from(format!("{}", $value));
            print!("{}={}", $name, escape(value));
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
