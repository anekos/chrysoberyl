
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

macro_rules! option {
    ( $condition:expr, $value:expr ) => {
        if $condition {
            Some($value)
        } else {
            None
        }
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
                Err(err) => puts_error!("reason" => s!(err))
            }
        }
    }
}

macro_rules! time {
    ($name:expr => $body:expr) => {
        {
            use std::time::Instant;
            use utils::duration_to_string;

            let t = Instant::now();
            let result = $body;
            let e = t.elapsed();
            info!("{}/time: {}", $name, duration_to_string(e));
            result
        }
    }
}

macro_rules! sprintln {
    ($name:expr, $fmt:expr) => {
        $name.push_str(concat!($fmt, "\n"));
    };
    ($name:expr, $fmt:expr $(,$args:expr)*) => {
        $name.push_str(&format!(concat!($fmt, "\n") $(,$args)*));
    }
}

macro_rules! sprint {
    ($name:expr, $fmt:expr) => {
        $name.push_str($fmt);
    };
    ($name:expr, $fmt:expr $(,$args:expr)*) => {
        $name.push_str(&format!($fmt $(,$args)*));
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
    ($var:ident = $value:expr, $else_value:expr) => {
        let $var = if let Some(it) = $value {
            it
        } else {
            return $else_value;
        };
    }
}

macro_rules! constant {
    ($value:expr) => {
        move |_| $value
    }
}
