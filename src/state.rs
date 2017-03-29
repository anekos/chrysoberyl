


pub struct States {
    pub status_bar: bool,
    pub reverse: bool,
    pub center_alignment: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateName {
    StatusBar,
    Reverse,
    CenterAlignment,
}


impl States {
    pub fn new() -> States {
        States { status_bar: false, reverse: false, center_alignment: false }
    }
}
