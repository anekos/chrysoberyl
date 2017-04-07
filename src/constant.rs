
pub static DEFAULT_TITLE: &'static str = env!("CARGO_PKG_NAME");
pub static DEFAULT_INFORMATION: &'static str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));
pub static VARIABLE_PREFIX: &'static str = "CHRYSOBERYL_";


pub fn env_name(name: &str) -> String {
    format!("{}{}", VARIABLE_PREFIX, name.to_uppercase())
}
