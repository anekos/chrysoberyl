

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
