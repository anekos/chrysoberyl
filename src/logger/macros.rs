
macro_rules! puts {
    ( $($name:expr => $value:expr),* ) => {
        {
            use logger;
            logger::puts(&[
                $( ($name.to_owned(), $value.to_owned()) ),*
            ])
        }
    }
}

macro_rules! puts_event {
    ( $event:expr  $(,$name:expr => $value:expr)* ) => {
        puts!("event" => $event $(, $name => $value)*)
    }
}

macro_rules! puts_error {
    ( $message:expr $(,$name:expr => $value:expr)* ) => {
        {
            use std::env;
            env::set_var("CHRY_LAST_ERROR", s!($message));
            puts!("event" => "error", "message" => $message $(, $name => $value)*)
        }
    }
}
