


pub struct States {
    pub status_bar: bool,
    pub reverse: bool,
    pub view: ViewState
}

pub struct ViewState {
    pub cols: usize,
    pub rows: usize,
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
        States {
            status_bar: false,
            reverse: false,
            view: ViewState {
                cols: 1,
                rows: 1,
                center_alignment: false,
            }
        }
    }
}
