
pub static DEFAULT_TITLE: &str = env!("CARGO_PKG_NAME");
pub static DEFAULT_INFORMATION: &str = concat!(env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION"));
pub static VARIABLE_PREFIX: &str = "CHRY_";
pub static OPTION_VARIABLE_PREFIX: &str = "CHRY_OPT_";
pub static USER_VARIABLE_PREFIX: &str = "CHRY_X_";
pub static APPLICATION_NAME: &str = env!("CARGO_PKG_NAME");
pub static README: &str = include_str!("../README.md");


pub fn env_name(name: &str) -> String {
    format!("{}{}", VARIABLE_PREFIX, name.to_uppercase())
}
