
use std::default::Default;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use cairo;
use gtk;

use app_path::{PathList, cache_dir};
use color::Color;
use entry::SearchKey;
use entry::filter::expression::Expr as FilterExpr;
use errors::ChryError;
use gui::{Views, Position};
use logger;
use option::OptionValue;
use remote_cache::curl_options::CurlOptions;
use size::{FitTo, Region};



pub struct States {
    pub abbrev_length: usize,
    pub auto_paging: AutoPaging,
    pub auto_reload: bool,
    pub curl_options: CurlOptions,
    pub drawing: DrawingState,
    pub go: Option<SearchKey>,
    pub history_file: Option<PathBuf>,
    pub idle_time: Duration,
    pub initial_position: Position,
    pub last_direction: Direction,
    pub last_filter: Filters,
    pub log_file: logger::file::File,
    pub operation_box: bool,
    pub path_list: PathList,
    pub pre_fetch: PreFetchState,
    pub empty_status_format: EmptyStatusFormat,
    pub reverse: bool,
    pub skip_resize_window: usize,
    pub spawned: bool,
    pub stable_push: bool,
    pub status_bar: bool,
    pub status_bar_align: Alignment,
    pub status_bar_height: Option<usize>,
    pub status_format: StatusFormat,
    pub stdout: logger::stdout::StdOut,
    pub title_format: TitleFormat,
    pub update_cache_atime: bool,
    pub view: Views,
    pub watch_files: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DrawingState {
    pub animation: bool,
    pub clipping: Option<Region>,
    pub fit_to: FitTo,
    pub link_color: Color,
    pub mask_operator: MaskOperator,
    pub rotation: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreFetchState {
    pub enabled: bool,
    pub limit_of_items: usize,
    pub page_size: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskOperator(pub cairo::Operator);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Alignment(pub gtk::Align);

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Forward,
    Backward
}

pub struct Filters {
    pub static_filter: Option<FilterExpr>,
    pub dynamic_filter: Option<FilterExpr>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum AutoPaging {
    DoNot,
    Always,
    Smart,
}


impl Default for States {
    fn default() -> Self {
        let mut history_file = cache_dir("history");
        history_file.push("input.log");

        States {
            abbrev_length: 30,
            auto_paging: AutoPaging::default(),
            auto_reload: false,
            curl_options: CurlOptions::default(),
            drawing: DrawingState::default(),
            empty_status_format: EmptyStatusFormat::default(),
            go: None,
            history_file: Some(history_file),
            idle_time: Duration::from_millis(250),
            initial_position: Position::default(),
            last_direction: Direction::Forward,
            last_filter: Filters::default(),
            log_file: logger::file::File::new(),
            operation_box: false,
            path_list: PathList::default(),
            pre_fetch: PreFetchState::default(),
            reverse: false,
            skip_resize_window: 0,
            spawned: false,
            stable_push: true,
            status_bar: true,
            status_bar_align: Alignment(gtk::Align::Center),
            status_bar_height: None,
            status_format: StatusFormat::default(),
            stdout: logger::stdout::StdOut::new(),
            title_format: TitleFormat::default(),
            update_cache_atime: false,
            view: Views::default(),
            watch_files: false,
        }
    }

}


impl Default for DrawingState {
    fn default() -> Self {
        DrawingState {
            animation: true,
            clipping: None,
            fit_to: FitTo::Cell,
            link_color: Color::new4(0, 0, 255, 32),
            mask_operator: MaskOperator(cairo::Operator::DestIn),
            rotation: 0,
        }
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


impl Default for Filters {
    fn default() -> Self {
        Filters {
            static_filter: None,
            dynamic_filter: None,
        }
    }
}

impl AutoPaging {
    pub fn enabled(&self) -> bool {
        AutoPaging::DoNot != *self
    }
}

impl fmt::Display for AutoPaging {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use state::AutoPaging::*;
        let result =
            match *self {
                DoNot => "no",
                Always => "always",
                Smart => "smart",
            };
        write!(f, "{}", result)
    }
}

impl Default for AutoPaging {
    fn default() -> Self {
        AutoPaging::DoNot
    }
}

macro_rules! gen_format {
    ($t:tt, $default:expr) => {

        #[derive(Clone, Debug, PartialEq)]
        pub enum $t {
            Literal(String),
            Script(String, String), /* filepath, source cache */
        }

        impl $t {
            pub fn generate(&self) -> String {
                use mruby::MRubyEnv;
                use shellexpand_wrapper as sh;

                match *self {
                    $t::Script(_, ref script) => {
                        MRubyEnv::generate_string(script).unwrap_or_else(|err| {
                            puts_error!(err, "at" => "generate/mruby_script");
                            o!("mruby script error")
                        })
                    },
                    $t::Literal(ref s) => sh::expand(s),
                }
            }
        }

        impl OptionValue for $t {
            fn set(&mut self, value: &str) -> Result<(), ChryError> {
                use shellexpand_wrapper as sh;

                if value.starts_with('@') {
                    let raw_path = &value[1..];
                    let path = sh::expand(raw_path);
                    let mut file = File::open(&path)?;
                    let mut script = o!("");
                    file.read_to_string(&mut script)?;
                    *self = $t::Script(o!(raw_path), script);
                } else {
                    *self = $t::Literal(o!(value));
                }
                Ok(())
            }

            fn unset(&mut self) -> Result<(), ChryError> {
                *self = $t::default();
                Ok(())
            }
        }

        impl Default for $t {
            fn default() -> Self {
                $t::Literal(o!($default))
            }
        }

        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                use self::$t::*;
                use util::shell::escape;

                match *self {
                    Script(ref filepath, _) => write!(f, "{}", escape(&format!("@{}", filepath))),
                    Literal(ref s) => write!(f, "{}", escape(s)),
                }
            }
        }
    }
}

gen_format!(
    StatusFormat,
    "<span background=\"red\">$CHRY_MESSAGE</span><span background=\"#005050\"> $CHRY_PAGING/$CHRY_PAGES </span> $CHRY_ABBREV_PATH <span foreground=\"grey\">$CHRY_FLAGS</span> <span foreground=\"rosybrown\">${CHRY_REMOTE_QUEUE}q${CHRY_REMOTE_BUFFER}b${CHRY_REMOTE_THREAD}t</span>");
gen_format!(
    EmptyStatusFormat,
    concat!("<span background=\"red\">$CHRY_MESSAGE</span>", env!("CARGO_PKG_NAME"), " v", env!("CARGO_PKG_VERSION")));
gen_format!(
    TitleFormat,
    "[$CHRY_PAGING/$CHRY_PAGES] $CHRY_PATH");
