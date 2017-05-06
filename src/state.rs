
use std::str::FromStr;
use std::default::Default;

use gdk_pixbuf::InterpType;

use entry::SearchKey;
use option;
use size::{FitTo, Region};


pub struct States {
    pub initialized: bool,
    pub status_bar: StatusBarValue,
    pub reverse: ReverseValue,
    pub auto_paging: AutoPagingValue,
    pub view: ViewState,
    pub show: Option<SearchKey>,
    pub status_format: String,
    pub drawing: DrawingOption,
    pub pre_fetch: Option<PreFetchState>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalingMethod(pub InterpType);

#[derive(Clone, Debug, PartialEq)]
pub struct DrawingOption {
    pub fit_to: FitTo,
    pub scaling: ScalingMethod,
    pub clipping: Option<Region>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreFetchState {
    pub page_size: usize,
    pub limit_of_items: usize,
}


pub const STATUS_FORMAT_DEFAULT: &'static str = "[$CHRYSOBERYL_PAGING/$CHRYSOBERYL_COUNT] $CHRYSOBERYL_PATH {$CHRYSOBERYL_FLAGS}";


impl Default for States {
    fn default() -> Self {
        States {
            initialized: false,
            status_bar: StatusBarValue::Enabled,
            reverse: ReverseValue::Disabled,
            auto_paging: AutoPagingValue::Disabled,
            status_format: o!(STATUS_FORMAT_DEFAULT),
            view: ViewState::default(),
            show: None,
            drawing: DrawingOption::default(),
            pre_fetch: Some(PreFetchState::default()),
        }
    }

}


impl Default for DrawingOption {
    fn default() -> Self {
        DrawingOption {
            fit_to: FitTo::Cell,
            scaling: ScalingMethod::default(),
            clipping: None,
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
    fn default() -> Self {
        ScalingMethod(InterpType::Bilinear)
    }
}


impl Default for PreFetchState {
    fn default() -> Self {
        PreFetchState {
            page_size: 5,
            limit_of_items: 100,
        }
    }
}


impl Default for ViewState {
    fn default() -> Self {
        ViewState {
            cols: 1,
            rows: 1,
            center_alignment: CenterAlignmentValue::Disabled,
        }
    }
}
