
use std::str::FromStr;
use std::default::Default;

use gdk_pixbuf::InterpType;

use size::FitTo;


pub struct States {
    pub initialized: bool,
    pub status_bar: bool,
    pub reverse: bool,
    pub auto_paging: bool,
    pub view: ViewState,
    pub fit_to: FitTo,
    pub scaling: ScalingMethod,
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
    AutoPaging,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScalingMethod(pub InterpType);


impl States {
    pub fn new() -> States {
        States {
            initialized: false,
            status_bar: false,
            reverse: false,
            fit_to: FitTo::Cell,
            auto_paging: false,
            view: ViewState {
                cols: 1,
                rows: 1,
                center_alignment: false,
            },
            scaling: ScalingMethod(InterpType::Bilinear)
        }
    }
}


impl FromStr for ScalingMethod {
    type Err = String;

    fn from_str(src: &str) -> Result<ScalingMethod, String> {
        match src {
            "n" | "nearest" => Ok(InterpType::Nearest),
            "t" | "tiles" => Ok(InterpType::Tiles),
            "b" | "bilinear" => Ok(InterpType::Bilinear),
            "h" | "hyper" => Ok(InterpType::Hyper),
            _ => Err(format!("Invalid scaling method name: {}", src))
        } .map(ScalingMethod)
    }
}

impl Default for ScalingMethod {
    fn default() -> ScalingMethod {
        ScalingMethod(InterpType::Bilinear)
    }
}
