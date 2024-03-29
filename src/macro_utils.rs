
macro_rules! max {
    ($x:expr) => {
        $x
    };
    ($x:expr $(,$ys:expr)*) => {
        {
            let (x, y) = ($x, max!($($ys),*));
            if x > y {
                x
            } else {
                y
            }
        }
    }
}

macro_rules! min {
    ($x:expr) => {
        $x
    };
    ($x:expr $(,$ys:expr)*) => {
        {
            let (x, y) = ($x, min!($($ys),*));
            if x < y {
                x
            } else {
                y
            }
        }
    }
}

macro_rules! not_implemented {
    () => {
        panic!("Not Implemented")
    }
}

macro_rules! o {
    ( $expr:expr ) => {
        $expr.to_owned()
    }
}

macro_rules! s {
    ( $expr:expr ) => {
        format!("{}", $expr)
    }
}

macro_rules! d {
    ( $expr:expr ) => {
        format!("{:?}", $expr)
    }
}

// for path debug out
macro_rules! p {
    ( $expr:expr ) => {
        $expr.to_str().map(|it| {
            format!("{}", it)
        }).unwrap_or_else(|| {
            d!($expr)
        })
    }
}

macro_rules! ignore {
    ( $body:expr )  => {
        {
            match $body {
                Ok(_) => (),
                Err(err) => puts_error!(err),
            }
        }
    }
}

macro_rules! through {
    ( [] $body:expr )  => {
        {
            $body
        }
    };
    ( [$name:ident = $e:expr $(,$rest_n:ident = $rest_e:expr)*] $body:expr )  => {
        {
            match $e {
                Ok($name) => through!([$($rest_n = $rest_e),*] $body),
                Err(err) => puts_error!(err),
            }
        }
    }
}

macro_rules! timeit {
    ($name:expr => $body:expr) => {
        {
            use std::time::Instant;
            use crate::util::time::duration_to_string;

            let t = Instant::now();
            let result = $body;
            log::info!("{}/time: {}", $name, duration_to_string(&t.elapsed()));
            result
        }
    }
}

macro_rules! sprintln {
    ($name:expr, $fmt:expr) => {
        $name.push_str(concat!($fmt, "\n"))
    };
    ($name:expr, $fmt:expr $(,$args:expr)*) => {
        $name.push_str(&format!(concat!($fmt, "\n") $(,$args)*))
    }
}

macro_rules! sprint {
    ($name:expr, $fmt:expr) => {
        $name.push_str($fmt)
    };
    ($name:expr, $fmt:expr $(,$args:expr)*) => {
        $name.push_str(&format!($fmt $(,$args)*))
    }
}

macro_rules! with_ouput_string {
    ($name:ident,  $body:expr) => {
        {
            let mut result: String = o!("");
            {
                let $name = &mut result;
                $body;
            }
            result
        }
    }
}

macro_rules! if_let_some {
    ($var:pat = $value:expr) => {
        if_let_some!($var = $value, ())
    };

    ($var:pat = $value:expr, $else_value:expr) => {
        #[allow(clippy::if_let_some_result)]
        let $var = if let Some(it) = $value {
            it
        } else {
            return $else_value;
        };
    }
}

macro_rules! if_let_ok {
    ($var:pat = $value:expr, $else_value:expr) => {
        let $var = match $value {
            Ok(it) => it,
            Err(err) => return $else_value(err),
        };
    }
}

macro_rules! count_idents {
    ($x:ident) => {
        1
    };
    ($x:ident $(,$xs:ident)*) => {
        1 + count_idents!($($xs),*)
    }
}

macro_rules! iterable_enum {
    ($name:ident => $($var:ident,)*) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum $name {
            $($var,)*
        }

        use std::slice::Iter;

        impl $name {
            pub fn iterator() -> Iter<'static, $name> {
                use self::$name::*;

                static ITEMS: [$name; count_idents!($($var),*)] = [$($var,)*];
                ITEMS.iter()
            }
        }
    }
}

macro_rules! clamp {
    ($min:expr, $value:expr, $max:expr) => {
        min!(max!($min, $value), $max)
    }
}

macro_rules! ok {
    ($value:expr) => {
        {
            $value;
            Ok(())
        }
    }
}

macro_rules! tap {
    ($name:ident = $value:expr, $block:expr) => {
        {
            let $name = $value;
            $block;
            $name
        }
    };

    (mut $name:ident = $value:expr, $block:expr) => {
        {
            let mut $name = $value;
            $block;
            $name
        }
    };

    ($value:expr, $block:expr) => {
        {
            let result = $value;
            $block;
            result
        }
    }
}

macro_rules! map {
    ($macro:tt $(,$values:expr)*) => {
        ($($macro!($values),)*)
    }
}
