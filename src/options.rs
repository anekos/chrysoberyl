


pub struct AppOptions {
    pub show_text: bool,
    pub reverse: bool,
    pub center_alignment: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppOptionName {
    ShowText,
    Reverse,
    CenterAlignment,
}


impl AppOptions {
    pub fn new() -> AppOptions {
        AppOptions { show_text: false, reverse: false, center_alignment: false }
    }
}
