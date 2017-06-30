
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use cmdline_parser::Parser;

use archive::ArchiveEntry;
use cherenkov::Filler;
use color::Color;
use entry::Meta;
use entry;
use expandable::Expandable;
use filer;
use gui::Direction;
use mapping;
use poppler;
use session::Session;
use size::Region;

mod parser;



#[derive(Clone, Debug)]
pub enum Operation {
    Cherenkov(CherenkovParameter),
    Clear,
    Clip(Region),
    Context(OperationContext, Box<Operation>),
    Count(Option<usize>),
    CountDigit(u8),
    DefineUserSwitch(String, Vec<Vec<String>>),
    Draw,
    Editor(Option<Expandable>, Vec<Expandable>, Vec<Session>),
    Expand(bool, Option<PathBuf>), /* recursive, base */
    First(Option<usize>, bool, MoveBy, bool), /* count, ignore-views, archive/page, wrap */
    Fill(Filler, Option<Region>, Color, bool, usize), /* region, mask, cell index */
    Filter(Box<Option<entry::filter::expression::Expr>>),
    Fragile(Expandable),
    Initialized,
    Input(mapping::Input),
    KillTimer(String),
    Last(Option<usize>, bool, MoveBy, bool),
    LazyDraw(u64, bool), /* serial, to_end */
    Load(PathBuf),
    LoadDefault,
    Map(MappingTarget, Vec<String>),
    MoveAgain(Option<usize>, bool, MoveBy, bool), /* count, ignore-views, archive/page, wrap */
    MoveEntry(entry::Position, entry::Position),
    Multi(VecDeque<Operation>, bool), /* operations, async */
    Next(Option<usize>, bool, MoveBy, bool),
    Nop,
    OperateFile(filer::FileOperation),
    PdfIndex(bool, bool, Vec<Expandable>, poppler::index::Format),
    PreFetch(u64),
    Previous(Option<usize>, bool, MoveBy, bool),
    PrintEntries,
    Pull,
    Push(Expandable, Option<Meta>, bool), /* path, meta, force */
    PushDirectory(Expandable, Option<Meta>, bool), /* path, meta, force */
    PushImage(Expandable, Option<Meta>, bool, Option<u8>), /* path, meta, force, expand-level */
    PushPdf(Expandable, Option<Meta>, bool),
    PushSibling(bool, Option<Meta>, bool, bool), /* next?, meta, force, show */
    PushURL(String, Option<Meta>, bool),
    Quit,
    Random,
    Refresh,
    ResetImage,
    Save(Option<PathBuf>, Vec<Session>),
    SearchText(Option<String>, bool), /* text, backward */
    Scroll(Direction, Vec<String>, f64), /* direction, operation, scroll_size_ratio */
    SetEnv(String, Option<Expandable>),
    Shell(bool, bool, Vec<Expandable>, Vec<Session>), /* async, operation, command_line, session */
    ShellFilter(Vec<Expandable>),
    Show(entry::SearchKey),
    Shuffle(bool), /* Fix current */
    Sort,
    TellRegion(f64, f64, f64, f64, u32), /* lef,t top, right, bottom, mousesbutton */
    Timer(String, Vec<String>, Duration, Option<usize>),
    Unclip,
    Undo(Option<usize>),
    UpdateOption(OptionName, OptionUpdater),
    UpdateUI,
    User(Vec<(String, String)>),
    Views(Option<usize>, Option<usize>),
    ViewsFellow(bool), /* for_rows */
    WindowResized,
    WithMessage(Option<String>, Box<Operation>),
    Write(PathBuf, Option<usize>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CherenkovParameter {
    pub radius: f64,
    pub random_hue: f64,
    pub n_spokes: usize,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationContext {
    pub input: mapping::Input,
    pub cell_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MappingTarget {
    Key(Vec<String>),
    Mouse(u32, Option<Region>),
    Event(String, Option<String>),
    Region(u32),
}

#[derive(Debug, PartialEq)]
pub enum ParsingError {
    NotOperation,
    InvalidOperation(String),
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum MoveBy {
    Page,
    Archive,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionUpdater {
    Set(String),
    Unset,
    Enable,
    Disable,
    Toggle,
    Cycle(bool), /* reverse */
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionName {
    PreDefined(PreDefinedOptionName),
    UserDefined(String),
}

iterable_enum!(PreDefinedOptionName =>
    AutoPaging,
    CenterAlignment,
    ColorError,
    ColorErrorBackground,
    ColorStatusBar,
    ColorStatusBarBackground,
    ColorWindowBackground,
    FitTo,
    HorizontalViews,
    MaskOperator,
    PreFetchEnabled,
    PreFetchLimit,
    PreFetchPageSize,
    Reverse,
    Scaling,
    StatusBar,
    StatusFormat,
    TitleFormat,
    VerticalViews,
);

#[derive(Clone, Debug, PartialEq)]
pub enum QueuedOperation {
    PushImage(PathBuf, Option<Meta>, bool, Option<u8>), /* path, meta, force, expand-level */
    PushDirectory(PathBuf, Option<Meta>, bool), /* path, meta, force */
    PushHttpCache(PathBuf, String, Option<Meta>, bool),
    PushArchiveEntry(PathBuf, ArchiveEntry, bool),
    PushPdfEntries(PathBuf, usize, Option<Meta>, bool), /* path, pages, meta, force */
}


impl FromStr for Operation {
    type Err = ParsingError;

    fn from_str(src: &str) -> Result<Self, ParsingError> {
        Operation::parse(src)
    }
}

impl FromStr for PreDefinedOptionName {
    type Err = ParsingError;

    fn from_str(src: &str) -> Result<Self, ParsingError> {
        use self::PreDefinedOptionName::*;

        match src {
            "auto-page" | "auto-paging" | "paging" => Ok(AutoPaging),
            "center" | "center-alignment"          => Ok(CenterAlignment),
            "fit" | "fit-to"                       => Ok(FitTo),
            "reverse" | "rev"                      => Ok(Reverse),
            "scaling"                              => Ok(Scaling),
            "status-bar" | "status"                => Ok(StatusBar),
            "status-format"                        => Ok(StatusFormat),
            "title-format"                         => Ok(TitleFormat),
            "pre-render"                           => Ok(PreFetchEnabled),
            "pre-render-limit"                     => Ok(PreFetchLimit),
            "pre-render-pages"                     => Ok(PreFetchPageSize),
            "vertical-views"                       => Ok(VerticalViews),
            "horizontal-views"                     => Ok(HorizontalViews),
            "mask-operator"                        => Ok(MaskOperator),
            "window-background-color"              => Ok(ColorWindowBackground),
            "status-bar-color"                     => Ok(ColorStatusBar),
            "status-bar-background-color"          => Ok(ColorStatusBarBackground),
            "error-color"                          => Ok(ColorError),
            "error-background-color"               => Ok(ColorErrorBackground),
            _                                      => Err(ParsingError::InvalidOperation(format!("Invalid option name: {}", src)))
        }
    }
}

impl FromStr for OptionName {
    type Err = ParsingError;

    fn from_str(src: &str) -> Result<Self, ParsingError> {
        use self::OptionName::*;

        Ok({
            src.parse().map(PreDefined).unwrap_or_else(|_| {
                UserDefined(o!(src))
            })
        })
    }
}

impl Default for OptionName {
    fn default() -> Self {
        OptionName::PreDefined(PreDefinedOptionName::StatusBar)
    }
}


impl Operation {
    pub fn parse_from_vec(whole: &[String]) -> Result<Operation, String> {
        _parse_from_vec(whole).map_err(|it| s!(it))
    }

    pub fn parse(s: &str) -> Result<Operation, ParsingError> {
        let ps: Vec<String> = Parser::new(s).map(|(_, it)| it).collect();
        _parse_from_vec(ps.as_slice())
    }

    pub fn parse_fuzziness(s: &str) -> Result<Operation, String> {
        match Operation::parse(s) {
            Err(ParsingError::InvalidOperation(err)) => Err(err),
            Err(ParsingError::NotOperation) => Ok(Operation::Push(Expandable(o!(s)), None, false)),
            Ok(op) => Ok(op)
        }
    }

    fn user(args: Vec<String>) -> Operation {
        let mut result: Vec<(String, String)> = vec![];
        let mut index = 0;

        for  arg in &args {
            let sep = arg.find('=').unwrap_or(0);
            let (key, value) = arg.split_at(sep);
            if key.is_empty() {
                result.push((format!("arg{}", index), value.to_owned()));
                index += 1;
            } else {
                result.push((key.to_owned(), value[1..].to_owned()));
            }
        }

        Operation::User(result)
    }
}


impl fmt::Display for ParsingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParsingError::InvalidOperation(ref err) =>
                write!(f, "Invalid operation: {}", err),
            ParsingError::NotOperation =>
                write!(f, "Not operation")
        }
    }
}



fn _parse_from_vec(whole: &[String]) -> Result<Operation, ParsingError> {
    use self::Operation::*;
    use filer::FileOperation::{Copy, Move};
    use self::parser::*;

    if let Some(head) = whole.get(0) {
        let name = &*head.to_lowercase();
        let args = &whole[1..];

        if name.starts_with('#') {
            return Ok(Nop)
        }

        if !(name.starts_with(';') || name.starts_with('@')) {
            return Err(ParsingError::NotOperation)
        }

        match name {
            "@cherenkov"                    => parse_cherenkov(whole),
            "@clear"                        => Ok(Clear),
            "@clip"                         => parse_clip(whole),
            "@copy-file"                    => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Copy(path, if_exist))),
            "@count"                        => parse_count(whole),
            "@cycle"                        => parse_option_cycle(whole),
            "@default"                      => Ok(LoadDefault),
            "@define-switch"                => parse_define_switch(whole),
            "@disable"                      => parse_option_1(whole, OptionUpdater::Disable),
            "@draw"                         => Ok(Draw),
            "@editor"                       => parse_editor(whole),
            "@enable"                       => parse_option_1(whole, OptionUpdater::Enable),
            "@entries"                      => Ok(PrintEntries),
            "@expand"                       => parse_expand(whole),
            "@fill"                         => parse_fill(whole),
            "@filter"                       => parse_filter(whole),
            "@first" | "@f"                 => parse_move(whole, First),
            "@fragile"                      => parse_command1(whole, |it| Fragile(Expandable(it))),
            "@input"                        => parse_input(whole),
            "@kill-timer"                   => parse_kill_timer(whole),
            "@last" | "@l"                  => parse_move(whole, Last),
            "@load"                         => parse_load(whole),
            "@map"                          => parse_map(whole),
            "@move-file"                    => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Move(path, if_exist))),
            "@move-again"                   => parse_move(whole, MoveAgain),
            "@move-entry"                   => parse_move_entry(whole),
            "@multi"                        => parse_multi(whole),
            "@next" | "@n"                  => parse_move(whole, Next),
            "@pdf-index"                    => parse_pdf_index(whole),
            "@prev" | "@p" | "@previous"    => parse_move(whole, Previous),
            "@push"                         => parse_push(whole, |it, meta, force| Push(Expandable(it), meta, force)),
            "@push-next"                    => parse_push_sibling(whole, true),
            "@push-image"                   => parse_push_image(whole),
            "@push-directory" | "@push-dir" => parse_push(whole, |it, meta, force| PushDirectory(Expandable(it), meta, force)),
            "@push-pdf"                     => parse_push(whole, |it, meta, force| PushPdf(Expandable(it), meta, force)),
            "@push-previous" | "@push-prev" => parse_push_sibling(whole, false),
            "@push-url"                     => parse_push(whole, PushURL),
            "@quit"                         => Ok(Quit),
            "@random" | "@rand"             => Ok(Random),
            "@refresh" | "@r"               => Ok(Refresh),
            "@reset-image"                  => Ok(ResetImage),
            "@save"                         => parse_save(whole),
            "@scroll"                       => parse_scroll(whole),
            "@search"                       => parse_search(whole),
            "@set"                          => parse_option_set(whole),
            "@set-env"                      => parse_set_env(whole),
            "@shell"                        => parse_shell(whole),
            "@shell-filter"                 => Ok(ShellFilter(whole.iter().map(|it| Expandable(it.clone())).collect())),
            "@show"                         => parse_show(whole),
            "@shuffle"                      => Ok(Shuffle(false)),
            "@sort"                         => Ok(Sort),
            "@timer"                        => parse_timer(whole),
            "@toggle"                       => parse_option_1(whole, OptionUpdater::Toggle),
            "@unclip"                       => Ok(Unclip),
            "@undo"                         => parse_undo(whole),
            "@unset"                        => parse_option_1(whole, OptionUpdater::Unset),
            "@user"                         => Ok(Operation::user(args.to_vec())),
            "@views"                        => parse_views(whole),
            "@write"                        => parse_write(whole),
            ";"                             => parse_multi_args(args, ";", true),
            _ => Err(format!("Unknown operation: {}", name))
        } .map_err(ParsingError::InvalidOperation)
    } else {
        Ok(Nop)
    }
}
