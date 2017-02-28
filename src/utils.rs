

macro_rules! through {
    ( [] $body:expr )  => {
        {
            $body
        }
    };
    ( [$name:ident = $e:expr $(,$rest_n:ident = $rest_e:expr)*] $body:expr )  => {
        {
            use output;

            match $e {
                Ok($name) => through!([$($rest_n = $rest_e),*] $body),
                Err(err) => output::error(err)
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
        }
    }
}

macro_rules! iter_let {
    ( $source:expr => [$($bindings:ident),*] $body:expr) => {
        {
            let mut iter = $source.iter();
            iter_let_inner!(iter => [$($bindings),*] $body)
        }
    }
}
