

macro_rules! define_from {
    ($type:ident) => {
        macro_rules! $type {
            ($value:expr) => {
                $type::from($value)
            }
        }
    }
}


define_from!(f64);
define_from!(f32);
define_from!(i64);
define_from!(u64);
