
macro_rules! iter_let {
    ( $source:ident => [$($bindings:ident),*] $body:expr) => {
        {
            let mut $source = $source.iter();
            iter_let_inner!($source => [$($bindings),*] $body)
        }
    }
}

macro_rules! iter_let_inner {
    ( $iter:ident => [] $body:expr ) => {
        $body
    };
    ( $iter:ident => [$binding:ident $(,$bindings:ident)*] $body:expr ) => {
        if let Some($binding) = $iter.next() {
            iter_let_inner!($iter => [$($bindings),*] $body)
        } else {
            None
        }
    }
}

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
            info!("Time\t{}\t{}", $name, duration_to_string(e));
            result
        }
    }
}
