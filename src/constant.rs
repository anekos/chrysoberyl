
pub static DEFAULT_TITLE: &'static str = env!("CARGO_PKG_NAME");
pub static DEFAULT_INFORMATION: &'static str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));
pub static VARIABLE_PREFIX: &'static str = "CHRY_";
pub static OPTION_VARIABLE_PREFIX: &'static str = "CHRY_OPT_";
pub static USER_VARIABLE_PREFIX: &'static str = "CHRY_X_";
pub static WINDOW_ROLE: &'static str = env!("CARGO_PKG_NAME");
pub static README: &'static str = include_str!("../README.md");


pub fn env_name(name: &str) -> String {
    format!("{}{}", VARIABLE_PREFIX, name.to_uppercase())
}
