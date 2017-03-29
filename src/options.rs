


pub struct AppOptions {
    pub status_bar: bool,
    pub reverse: bool,
    pub center_alignment: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppOptionName {
    StatusBar,
    Reverse,
    CenterAlignment,
}


impl AppOptions {
    pub fn new() -> AppOptions {
        AppOptions { status_bar: false, reverse: false, center_alignment: false }
    }
}
