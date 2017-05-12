
use std::default::Default;

use gdk_pixbuf::InterpType;

use entry::SearchKey;
use size::{FitTo, Region};


pub struct States {
    pub initialized: bool,
    pub status_bar: bool,
    pub reverse: bool,
    pub auto_paging: bool,
    pub view: ViewState,
    pub show: Option<SearchKey>,
    pub status_format: String,
    pub drawing: DrawingOption,
    pub pre_fetch: Option<PreFetchState>,
}

pub struct ViewState {
    pub cols: usize,
    pub rows: usize,
    pub center_alignment: bool,
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
            status_bar: true,
            reverse: false,
            auto_paging: false,
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
            center_alignment: false,
        }
    }
}
