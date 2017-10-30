
use std::ops::Range;



#[inline]
pub fn feq(x: f64, y: f64, error: f64) -> bool {
    (x - y).abs() < error
}

pub fn range_contains<T: PartialOrd>(range: &Range<T>, index: &T) -> bool {
    range.start <= *index && *index <= range.end
}
