
macro_rules! puts {
    ( $($name:expr => $value:expr),* ) => {
        {
            use crate::logger;
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
