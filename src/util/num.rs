
use std::ops::Range;



#[inline]
pub fn feq(x: f64, y: f64, error: f64) -> bool {
    (x - y).abs() < error
}

pub fn range_contains<T: PartialOrd>(range: &Range<T>, index: &T) -> bool {
    range.start <= *index && *index <= range.end
}

macro_rules! cycle_uint {
    ( $type:ty, $reverse:expr, $n:expr, $target:ident ) => {
        {
            let reverse = $reverse;
            let n = $n as $type;
            if reverse {
                $target.wrapping_sub(n)
            } else {
                $target.wrapping_add(n)
            }
        }
    }
}


#[cfg(test)]#[test]
fn test_cycle_uint() {
    fn cycle(x: u8, reverse: bool, n: usize) -> u8 {
        cycle_uint!(u8, reverse, n, x)
    }

    assert_eq!(256usize as u8, 0);
    assert_eq!(257usize as u8, 1);

    assert_eq!(cycle(1, false, 1), 2);
    assert_eq!(cycle(255, false, 1), 0);
    assert_eq!(cycle(255, false, 2), 1);
    assert_eq!(cycle(0, false, 255), 255);
    assert_eq!(cycle(0, false, 256), 0);
}
