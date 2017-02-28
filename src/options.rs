



pub struct AppOptions {
    pub show_text: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppOptionName {
    ShowText
}


impl AppOptions {
    pub fn new() -> AppOptions {
        AppOptions { show_text: false }
    }
}
