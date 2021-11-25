
use std::default::Default;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use crate::app_path::{PathList, cache_dir};
use crate::cherenkov::Operator;
use crate::color::Color;
use crate::entry::SearchKey;
use crate::entry::filter::expression::Expr as FilterExpr;
use crate::errors::AppResultU;
use crate::expandable::Expandable;
use crate::gui::{Position, Screen, Views};
use crate::logger;
use crate::option::OptionValue;
use crate::remote_cache::curl_options::CurlOptions;
use crate::size::{FitTo, Region};
use crate::util::string::remove_linebreaks;



pub struct States {
    pub abbrev_length: usize,
    pub auto_paging: AutoPaging,
    pub auto_reload: bool,
    pub canonicalize: bool,
    pub curl_options: CurlOptions,
    pub drawing: Drawing,
    pub empty_status_format: EmptyStatusFormat,
    pub freezed: bool,
    pub go: Option<SearchKey>,
    pub history_file: Option<PathBuf>,
    pub idle_time: Duration,
    pub ignore_failures: bool,
    pub initial_position: Position,
    pub last_direction: Direction,
    pub last_filter: Filters,
    pub log_file: logger::file::File,
    pub path_list: PathList,
    pub pre_fetch: PreFetchState,
    pub reverse: bool,
    pub screen: Screen,
    pub skip_resize_window: usize,
    pub spawned: bool,
    pub stable_push: bool,
    pub status_bar: bool,
    pub status_bar_align: Alignment,
    pub status_bar_height: Option<usize>,
    pub status_bar_overlay: bool,
    pub status_format: StatusFormat,
    pub stdout: logger::stdout::StdOut,
    pub style: Style,
    pub time_to_hide_pointer: Option<u32>,
    pub title_format: TitleFormat,
    pub update_cache_atime: bool,
    pub view: Views,
    pub watch_files: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Drawing {
    pub animation: bool,
    pub clipping: Option<Region>,
    pub fit_to: FitTo,
    pub horizontal_flip: bool,
    pub link_color: Color,
    pub mask_operator: Operator,
    pub rotation: u8,
    pub vertical_flip: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreFetchState {
    pub cache_stages: usize,
    pub enabled: bool,
    pub limit_of_items: usize,
    pub page_size: usize,
}

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
            canonicalize: true,
            curl_options: CurlOptions::default(),
            drawing: Drawing::default(),
            empty_status_format: EmptyStatusFormat::default(),
            freezed: false,
            go: None,
            history_file: Some(history_file),
            idle_time: Duration::from_millis(250),
            ignore_failures: true,
            initial_position: Position::default(),
            last_direction: Direction::Forward,
            last_filter: Filters::default(),
            log_file: logger::file::File::new(),
            path_list: PathList::default(),
            pre_fetch: PreFetchState::default(),
            reverse: false,
            screen: Screen::Main,
            skip_resize_window: 0,
            spawned: false,
            stable_push: true,
            status_bar: true,
            status_bar_align: Alignment(gtk::Align::Center),
            status_bar_height: None,
            status_bar_overlay: false,
            status_format: StatusFormat::default(),
            stdout: logger::stdout::StdOut::new(),
            style: Style::default(),
            time_to_hide_pointer: Some(1000),
            title_format: TitleFormat::default(),
            update_cache_atime: false,
            view: Views::default(),
            watch_files: false,
        }
    }

}


impl Default for Drawing {
    fn default() -> Self {
        Drawing {
            animation: true,
            clipping: None,
            fit_to: FitTo::Cell,
            horizontal_flip: false,
            link_color: Color::new4(0, 0, 255, 32),
            mask_operator: Operator(cairo::Operator::DestIn),
            rotation: 0,
            vertical_flip: false,
        }
    }
}


impl Default for PreFetchState {
    fn default() -> Self {
        PreFetchState {
            cache_stages: 2,
            enabled: true,
            page_size: 5,
            limit_of_items: 10,
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
    pub fn enabled(self) -> bool {
        AutoPaging::DoNot != self
    }
}

impl fmt::Display for AutoPaging {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::state::AutoPaging::*;
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

macro_rules! gen_includable {
    ($t:tt, $default:expr) => {

        #[derive(Clone, Debug, PartialEq)]
        pub enum $t {
            Literal(String),
            Script(Expandable, String), /* filepath, source cache */
        }

        impl OptionValue for $t {
            fn set(&mut self, value: &str) -> AppResultU {
                if let Some(raw_path) = value.strip_prefix('@') {
                    let path = Expandable::new(o!(raw_path));
                    let mut file = File::open(&path.to_string())?;
                    let mut script = o!("");
                    file.read_to_string(&mut script)?;
                    *self = $t::Script(path, script);
                } else {
                    *self = $t::Literal(remove_linebreaks(value));
                }
                Ok(())
            }

            fn unset(&mut self) -> AppResultU {
                *self = $t::default();
                Ok(())
            }
        }

        impl Default for $t {
            fn default() -> Self {
                let mut result = $t::Literal(o!(""));
                let _ = result.set($default);
                result
            }
        }

        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                use self::$t::*;

                match *self {
                    Script(ref filepath, _) => write!(f, "{}", format!("@{}", filepath.as_raw())),
                    Literal(ref s) => write!(f, "{}", s),
                }
            }
        }
    }
}

macro_rules! gen_format {
    ($t:tt, $default:expr) => {
        gen_includable!($t, $default);

        impl $t {
            pub fn generate(&self) -> String {
                use crate::shellexpand_wrapper as sh;

                match *self {
                    $t::Script(_, _) => "NOT IMPLEMENTED".to_owned(),
                    $t::Literal(ref s) => sh::expand(s),
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
gen_includable!(
    Style,
    include_str!("static/default.css"));
