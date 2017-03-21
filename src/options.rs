


pub struct AppOptions {
    pub show_text: bool,
    pub reverse: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppOptionName {
    ShowText,
    Reverse
}


impl AppOptions {
    pub fn new() -> AppOptions {
        AppOptions { show_text: false, reverse: false }
    }
}
