
pub static DEFAULT_TITLE: &'static str = env!("CARGO_PKG_NAME");
pub static DEFAULT_INFORMATION: &'static str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));
pub static VARIABLE_PREFIX: &'static str = "CHRYSOBERYL_";


pub fn prefixed(name: &str) -> String {
    format!("{}{}", VARIABLE_PREFIX, name)
}
