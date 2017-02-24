
use gdk;



pub fn to_name(keyval: u32) -> String {
    gdk::keyval_name(keyval).unwrap_or(format!("{}", keyval))
}
