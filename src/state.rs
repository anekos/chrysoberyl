
use std::default::Default;

use cairo;
use gdk_pixbuf::InterpType;

use entry::SearchKey;
use entry::filter::expression::Expr as FilterExpr;
use logger;
use size::{FitTo, Region};



pub struct States {
    pub initialized: bool,
    pub spawned: bool,
    pub status_bar: bool,
    pub reverse: bool,
    pub auto_paging: bool,
    pub view: ViewState,
    pub go: Option<SearchKey>,
    pub status_format: StatusFormat,
    pub title_format: TitleFormat,
    pub drawing: DrawingState,
    pub pre_fetch: PreFetchState,
    pub last_direction: Direction,
    pub last_filter: Filters,
    pub log_file: logger::file::File,
    pub stdout: Option<logger::Handle>,
    pub abbrev_length: usize,
}

pub struct ViewState {
    pub cols: usize,
    pub rows: usize,
    pub center_alignment: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalingMethod(pub InterpType);

#[derive(Clone, Debug, PartialEq)]
pub struct DrawingState {
    pub fit_to: FitTo,
    pub scaling: ScalingMethod,
    pub clipping: Option<Region>,
    pub mask_operator: MaskOperator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreFetchState {
    pub enabled: bool,
    pub page_size: usize,
    pub limit_of_items: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StatusFormat(pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct TitleFormat(pub String);

#[derive(Clone, Debug, PartialEq)]
pub struct MaskOperator(pub cairo::Operator);

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Forward,
    Backward
}

pub struct Filters {
    pub static_filter: Option<FilterExpr>,
    pub dynamic_filter: Option<FilterExpr>,
}


impl Default for States {
    fn default() -> Self {
        States {
            initialized: false,
            spawned: false,
            status_bar: true,
            reverse: false,
            auto_paging: false,
            status_format: StatusFormat::default(),
            title_format: TitleFormat::default(),
            view: ViewState::default(),
            go: None,
            drawing: DrawingState::default(),
            pre_fetch: PreFetchState::default(),
            last_direction: Direction::Forward,
            last_filter: Filters::default(),
            log_file: logger::file::File::new(),
            stdout: None,
            abbrev_length: 30,
        }
    }

}


impl Default for DrawingState {
    fn default() -> Self {
        DrawingState {
            fit_to: FitTo::Cell,
            scaling: ScalingMethod::default(),
            clipping: None,
            mask_operator: MaskOperator(cairo::Operator::DestIn),
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
            enabled: true,
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

impl ViewState {
    pub fn len(&self) -> usize {
        self.rows * self.cols
    }
}


impl Default for StatusFormat {
    fn default() -> Self {
        StatusFormat(o!("<span background=\"red\">$CHRY_MESSAGE</span><span background=\"#005050\"> $CHRY_PAGING/$CHRY_COUNT </span> $CHRY_PATH <span foreground=\"grey\">$CHRY_FLAGS</span>"))
    }
}

impl Default for TitleFormat {
    fn default() -> Self {
        TitleFormat(o!("[$CHRY_PAGING/$CHRY_COUNT] $CHRY_PATH"))
    }
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            static_filter: None,
            dynamic_filter: None,
        }
    }
}
