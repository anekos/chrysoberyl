
use std::time::Duration;
use std::path::PathBuf;



pub fn path_to_str(path: &PathBuf) -> &str {
    path.to_str().unwrap()
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
                Err(err) => puts_error!("reason" => err)
            }
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

macro_rules! iter_let {
    ( $source:ident => [$($bindings:ident),*] $body:expr) => {
        {
            let mut $source = $source.iter();
            iter_let_inner!($source => [$($bindings),*] $body)
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


pub fn duration_to_string(t: Duration) -> String {
    let msec: u64 = t.as_secs() * 1000 + t.subsec_nanos() as u64 / 1000000;

    if 60 * 1000 <= msec {
        format!("{} min {} sec", msec / 60 / 1000, msec % (60 * 1000) / 1000)
    } else {
        format!("{} sec", msec as f64 / 1000.0)
    }
}
