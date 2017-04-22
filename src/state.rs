
use std::str::FromStr;
use std::default::Default;

use gdk_pixbuf::InterpType;

use size::FitTo;
use option;


pub struct States {
    pub initialized: bool,
    pub status_bar: StatusBarValue,
    pub reverse: ReverseValue,
    pub auto_paging: AutoPagingValue,
    pub view: ViewState,
    pub fit_to: FitTo,
    pub scaling: ScalingMethod,
}

boolean_option!(StatusBarValue, STATUS_BAR_DEFAULT, 's', 'S');
boolean_option!(ReverseValue, REVERSE_DEFAULT, 'r', 'R');
boolean_option!(AutoPagingValue, AUTO_PAGING_DEFAULT, 'a', 'A');
boolean_option!(CenterAlignmentValue, CENTER_ALIGNMENT_VALUE, 'c', 'C');

pub struct ViewState {
    pub cols: usize,
    pub rows: usize,
    pub center_alignment: CenterAlignmentValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateName {
    StatusBar,
    Reverse,
    CenterAlignment,
    AutoPaging,
    FitTo,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScalingMethod(pub InterpType);


impl States {
    pub fn new() -> States {
        States {
            initialized: false,
            status_bar: StatusBarValue::Enabled,
            reverse: ReverseValue::Disabled,
            auto_paging: AutoPagingValue::Disabled,
            fit_to: FitTo::Cell,
            view: ViewState {
                cols: 1,
                rows: 1,
                center_alignment: CenterAlignmentValue::Disabled,
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
