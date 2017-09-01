
use std::default::Default;

use cairo;
use gdk_pixbuf::InterpType;

use app_path::PathList;
use entry::SearchKey;
use entry::filter::expression::Expr as FilterExpr;
use logger;
use remote_cache::curl_options::CurlOptions;
use size::{FitTo, Region};



pub struct States {
    pub abbrev_length: usize,
    pub auto_paging: bool,
    pub curl_options: CurlOptions,
    pub drawing: DrawingState,
    pub go: Option<SearchKey>,
    pub last_direction: Direction,
    pub last_filter: Filters,
    pub log_file: logger::file::File,
    pub path_list: PathList,
    pub pre_fetch: PreFetchState,
    pub reverse: bool,
    pub skip_resize_window: usize,
    pub spawned: bool,
    pub status_bar: bool,
    pub status_format: StatusFormat,
    pub stdout: Option<logger::Handle>,
    pub title_format: TitleFormat,
    pub update_cache_atime: bool,
    pub view: ViewState,
}

pub struct ViewState {
    pub center_alignment: bool,
    pub cols: usize,
    pub rows: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalingMethod(pub InterpType);

#[derive(Clone, Debug, PartialEq)]
pub struct DrawingState {
    pub clipping: Option<Region>,
    pub fit_to: FitTo,
    pub mask_operator: MaskOperator,
    pub scaling: ScalingMethod,
    pub rotation: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreFetchState {
    pub enabled: bool,
    pub limit_of_items: usize,
    pub page_size: usize,
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
            abbrev_length: 30,
            auto_paging: false,
            curl_options: CurlOptions::default(),
            drawing: DrawingState::default(),
            go: None,
            last_direction: Direction::Forward,
            last_filter: Filters::default(),
            log_file: logger::file::File::new(),
            path_list: PathList::default(),
            pre_fetch: PreFetchState::default(),
            reverse: false,
            skip_resize_window: 0,
            spawned: false,
            status_bar: true,
            status_format: StatusFormat::default(),
            stdout: None,
            title_format: TitleFormat::default(),
            update_cache_atime: false,
            view: ViewState::default(),
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
            rotation: 0,
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
        StatusFormat(o!("<span background=\"red\">$CHRY_MESSAGE</span><span background=\"#005050\"> $CHRY_PAGING/$CHRY_PAGES </span> $CHRY_ABBREV_PATH <span foreground=\"grey\">$CHRY_FLAGS</span> <span foreground=\"rosybrown\">${CHRY_REMOTE_QUEUE}q${CHRY_REMOTE_BUFFER}b${CHRY_REMOTE_THREAD}t</span>"))
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
