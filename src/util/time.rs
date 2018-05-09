
use std::time::Duration;



pub fn duration_to_string(t: &Duration) -> String {
    let msec: u64 = t.as_secs() * 1000 + u64!(t.subsec_nanos()) / 1_000_000;

    if 60 * 1000 <= msec {
        format!("{} min {} sec", msec / 60 / 1000, msec % (60 * 1000) / 1000)
    } else {
        format!("{} sec", msec as f64 / 1000.0)
    }
}

pub fn duration_to_seconds(t: &Duration) -> f64 {
    t.as_secs() as f64 + t.subsec_nanos() as f64 * 1e-9
}
