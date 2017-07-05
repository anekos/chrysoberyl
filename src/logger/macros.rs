
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
    ( $($name:expr => $value:expr),* ) => {
        puts!("event" => "error" $(, $name => $value)*)
    }
}
