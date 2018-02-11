
use std::collections::{HashMap, VecDeque};
use std::error;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use cmdline_parser::Parser;

use archive::ArchiveEntry;
use cherenkov::fill::Shape;
use color::Color;
use command_line;
use controller;
use entry::filter::expression::Expr as FilterExpr;
use entry::{Meta, EntryType};
use entry;
use errors::ChryError;
use events::EventName;
use expandable::Expandable;
use filer;
use gui::Direction;
use key::Key;
use key::KeySequence;
use mapping;
use poppler;
use session::Session;
use size::Region;

mod parser;
pub mod option;

use self::option::{OptionName, OptionUpdater};



#[derive(Clone)]
pub enum Operation {
    AppEvent(EventName, HashMap<String, String>),
    ChangeDirectory(String),
    Cherenkov(CherenkovParameter),
    Clear,
    Clip(Region),
    Context(OperationContext, Box<Operation>),
    Controller(controller::Source),
    Count(Option<usize>),
    CountDigit(u8),
    DefineUserSwitch(String, Vec<Vec<String>>),
    Delete(Box<entry::filter::expression::Expr>),
    Draw,
    Editor(Option<Expandable>, Vec<Expandable>, Vec<Session>),
    Error(String),
    Expand(bool, Option<PathBuf>), /* recursive, base */
    FileChanged(PathBuf),
    Fill(Shape, Option<Region>, Color, bool, usize), /* region, mask, cell index */
    First(Option<usize>, bool, MoveBy, bool), /* count, ignore-views, archive/page, wrap */
    Filter(bool, Box<Option<entry::filter::expression::Expr>>), /* dynamic, filter expression */
    FlyLeaves(usize),
    Go(entry::SearchKey),
    Input(mapping::Input),
    InitialProcess(Vec<command_line::Entry>, bool, bool), /* command_lin::entries, shuffle, stdin_as_binary */
    Jump(Expandable), /* marker name */
    KillTimer(String),
    Last(Option<usize>, bool, MoveBy, bool),
    LazyDraw(u64, bool), /* serial, to_end */
    Load(Expandable, bool), /* path, search_path */
    LoadDefault,
    MakeVisibles(Vec<Option<Region>>),
    Map(MappingTarget, Option<usize>, Vec<String>), /* target, remain, operation */
    Mark(Expandable, Option<entry::SearchKey>),
    Meow,
    Message(Option<String>),
    MoveAgain(Option<usize>, bool, MoveBy, bool), /* count, ignore-views, archive/page, wrap */
    Multi(VecDeque<Operation>, bool), /* operations, async */
    Next(Option<usize>, bool, MoveBy, bool),
    Nop,
    OperateFile(filer::FileOperation),
    OperationEntry(OperationEntryAction),
    Page(usize),
    PdfIndex(bool, bool, bool, Vec<Expandable>, poppler::index::Format, Option<String>), /* async, read_operations, search_path, ... */
    PreFetch(u64),
    Previous(Option<usize>, bool, MoveBy, bool),
    Pull,
    Push(Expandable, Option<Meta>, bool), /* path, meta, force */
    PushArchive(Expandable, Option<Meta>, bool), /* path, meta, force */
    PushDirectory(Expandable, Option<Meta>, bool), /* path, meta, force */
    PushImage(Expandable, Option<Meta>, bool, Option<u8>), /* path, meta, force, expand-level */
    PushMemory(Vec<u8>), /* memory */
    PushPdf(Expandable, Option<Meta>, bool),
    PushSibling(bool, Option<Meta>, bool, bool), /* next?, meta, force, show */
    PushURL(String, Option<Meta>, bool, Option<EntryType>),
    Query(Vec<String>, Option<String>), /* operation, caption */
    Random,
    Refresh(bool), /* image_cache */
    ResetImage,
    ResetScrolls(bool), /* to_end */
    Save(PathBuf, Vec<Session>),
    SearchText(Option<String>, bool, Color), /* text, backward */
    Scroll(Direction, Vec<String>, f64, bool), /* direction, operation, scroll_size_ratio, crush */
    SetEnv(String, Option<Expandable>),
    Shell(bool, bool, bool, Vec<Expandable>, Vec<Session>), /* async, operation, search_path, command_line, session */
    ShellFilter(Vec<Expandable>, bool), /* path, search_path */
    Show(Option<usize>, bool, MoveBy, bool), /* count, ignore-views, archive/page, wrap */
    Shuffle(bool), /* Fix current */
    Sort(bool, SortKey, bool), /* fix, key, reverse */
    TellRegion(f64, f64, f64, f64, Key), /* lef,t top, right, bottom, mousesbutton */
    Timer(String, Vec<String>, Duration, Option<usize>),
    Unclip,
    Undo(Option<usize>),
    Unmap(MappingTarget),
    Unmark(Option<Expandable>), /* all or given */
    Update(Updated),
    UpdateOption(OptionName, OptionUpdater),
    UpdateUI,
    User(Vec<(String, String)>),
    Views(Option<usize>, Option<usize>),
    ViewsFellow(bool), /* for_rows */
    When(FilterExpr, bool, Vec<String>), /* filter, reverse(unless), operation */
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
    Unified(KeySequence, Option<Region>),
    Event(Option<EventName>, Option<String>),
    Region(Key),
}

#[derive(Debug, PartialEq)]
pub enum ParsingError {
    NotOperation(String),
    InvalidArgument(String),
    Fixed(&'static str),
    TooFewArguments,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum MoveBy {
    Page,
    Archive,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QueuedOperation {
    PushArchive(PathBuf, Option<Meta>, bool, Option<String>), /* path, meta, force, remote-url */
    PushArchiveEntry(PathBuf, ArchiveEntry, Option<Meta>, bool, Option<String>), /* path, archive-entry, meta, force, remote-url */
    PushDirectory(PathBuf, Option<Meta>, bool), /* path, meta, force */
    PushImage(PathBuf, Option<Meta>, bool, Option<u8>, Option<String>), /* path, meta, force, expand-level, remote-url */
    PushMemory(Vec<u8>), /* memory */
    PushPdf(PathBuf, Option<Meta>, bool, Option<String>), /* path, meta, force, remote-url */
    PushPdfEntries(PathBuf, usize, Option<Meta>, bool, Option<String>), /* path, pages, meta, force, remote-url */
}

#[derive(Default, Debug, Clone)]
pub struct Updated {
    pub counter: bool,
    pub pointer: bool,
    pub label: bool,
    pub image: bool,
    pub image_options: bool,
    pub message: bool,
    pub target_regions: Option<Vec<Option<Region>>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortKey {
    Natural,
    Accessed,
    Created,
    Modified,
    FileSize,
    Dimensions,
    Width,
    Height,
}

#[derive(Clone, Copy)]
pub enum OperationEntryAction {
    Open,
    SendOperation,
    Close,
}


impl FromStr for Operation {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
        Operation::parse(src)
    }
}


fn _parse_from_vec(whole: &[String]) -> Result<Operation, ParsingError> {
    use self::Operation::*;
    use self::parser::*;

    if let Some(head) = whole.get(0) {
        let name = &*head.to_lowercase();
        let args = &whole[1..];

        if name.starts_with('#') {
            return Ok(Nop)
        }

        if !(name.starts_with(';') || name.starts_with('@')) {
            return Err(ParsingError::NotOperation(o!(name)))
        }

        match name {
            ";"                             => parse_multi_args(args, ";", true),
            "@cd" | "@chdir" | "@change-directory"
                                            => parse_command1(whole, Operation::ChangeDirectory),
            "@cherenkov"                    => parse_cherenkov(whole),
            "@clear"                        => Ok(Clear),
            "@clip"                         => parse_clip(whole),
            "@controller" | "@control"      => parse_controller(whole),
            "@count"                        => parse_count(whole),
            "@cycle"                        => parse_option_cycle(whole),
            "@dec" | "@decrement" | "@decrease" | "@--"
                                            => parse_usize(whole, OptionUpdater::Decrement, 10),
            "@default"                      => Ok(LoadDefault),
            "@define-switch"                => parse_define_switch(whole),
            "@delete"                       => parse_delete(whole),
            "@disable"                      => parse_option_1(whole, OptionUpdater::Disable),
            "@draw"                         => Ok(Draw),
            "@editor"                       => parse_editor(whole),
            "@enable"                       => parse_option_1(whole, OptionUpdater::Enable),
            "@entry"                        => parse_operation_entry(whole),
            "@expand"                       => parse_expand(whole),
            "@file"                         => parse_file(whole),
            "@fill"                         => parse_fill(whole),
            "@filter"                       => parse_filter(whole),
            "@first" | "@f"                 => parse_move(whole, First),
            "@fly-leaves"                   => parse_fly_leaves(whole),
            "@go"                           => parse_go(whole),
            "@inc" | "@increment" | "@increase" | "@++"
                                            => parse_usize(whole, OptionUpdater::Increment, 10),
            "@input"                        => parse_input(whole),
            "@jump"                         => parse_command1(whole, Jump),
            "@kill-timer"                   => parse_kill_timer(whole),
            "@last" | "@l"                  => parse_move(whole, Last),
            "@load"                         => parse_load(whole),
            "@map"                          => parse_map(whole, true),
            "@mark"                         => parse_mark(whole),
            "@meow"                         => Ok(Meow),
            "@message"                      => parse_message(whole),
            "@move-again"                   => parse_move(whole, MoveAgain),
            "@multi"                        => parse_multi(whole),
            "@next" | "@n"                  => parse_move(whole, Next),
            "@nop"                          => Ok(Nop),
            "@page"                         => parse_page(whole),
            "@pdf-index"                    => parse_pdf_index(whole),
            "@prev" | "@p" | "@previous"    => parse_move(whole, Previous),
            "@push"                         => parse_push(whole, |it, meta, force| Push(Expandable::new(it), meta, force)),
            "@push-archive"                 => parse_push(whole, |it, meta, force| PushArchive(Expandable::new(it), meta, force)),
            "@push-directory" | "@push-dir" => parse_push(whole, |it, meta, force| PushDirectory(Expandable::new(it), meta, force)),
            "@push-image"                   => parse_push_image(whole),
            "@push-next"                    => parse_push_sibling(whole, true),
            "@push-pdf"                     => parse_push(whole, |it, meta, force| PushPdf(Expandable::new(it), meta, force)),
            "@push-previous" | "@push-prev" => parse_push_sibling(whole, false),
            "@push-url"                     => parse_push_url(whole),
            "@query"                        => parse_query(whole),
            "@quit"                         => Ok(EventName::Quit.operation()),
            "@random" | "@rand"             => Ok(Random),
            "@refresh" | "@r"               => parse_refresh(whole),
            "@reset-image"                  => Ok(ResetImage),
            "@save"                         => parse_save(whole),
            "@scroll"                       => parse_scroll(whole),
            "@search"                       => parse_search(whole),
            "@set"                          => parse_option_set(whole),
            "@set-env"                      => parse_set_env(whole),
            "@set-by-count" | "@set-count"  => parse_option_1(whole, OptionUpdater::SetByCount),
            "@shell"                        => parse_shell(whole),
            "@shell-filter"                 => parse_shell_filter(whole),
            "@show"                         => parse_move(whole, Show),
            "@shuffle"                      => parse_modify_entry_order(whole, Operation::Shuffle),
            "@sort"                         => parse_sort(whole),
            "@timer"                        => parse_timer(whole),
            "@toggle"                       => parse_option_1(whole, OptionUpdater::Toggle),
            "@unclip"                       => Ok(Unclip),
            "@undo"                         => parse_undo(whole),
            "@unless"                       => parse_when(whole, true),
            "@unmap"                        => parse_map(whole, false),
            "@unmark"                       => parse_command1(whole, |it| Unmark(Some(it))),
            "@unmark-all"                   => Ok(Unmark(None)),
            "@unset"                        => parse_option_1(whole, OptionUpdater::Unset),
            "@update"                       => parse_update(whole),
            "@user"                         => Ok(Operation::user(args)),
            "@views"                        => parse_views(whole),
            "@when"                         => parse_when(whole, false),
            "@write"                        => parse_write(whole),
            _ => Err(ParsingError::NotOperation(o!(name)))
        }
    } else {
        Ok(Nop)
    }
}

fn _parse_from_str(s: &str) -> Result<Operation, ParsingError> {
    let ps: Vec<String> = Parser::new(s).map(|(_, it)| it).collect();
    _parse_from_vec(ps.as_slice())
}


impl Operation {
    pub fn parse_from_vec(whole: &[String]) -> Result<Operation, ChryError> {
        _parse_from_vec(whole).map_err(ChryError::from)
    }

    pub fn parse(s: &str) -> Result<Operation, ChryError> {
        _parse_from_str(s).map_err(ChryError::from)
    }

    pub fn parse_fuzziness(s: &str) -> Result<Operation, ChryError> {
        match _parse_from_str(s) {
            Err(ParsingError::NotOperation(_)) => Ok(Operation::Push(Expandable::new(o!(s)), None, false)),
            Err(err) => Err(ChryError::from(err)),
            Ok(op) => Ok(op)
        }
    }

    fn user(args: &[String]) -> Operation {
        let mut result: Vec<(String, String)> = vec![];
        let mut index = 0;

        for  arg in args {
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
        use self::ParsingError::*;

        match *self {
            Fixed(err) =>
                write!(f, "{}", err),
            InvalidArgument(ref err) =>
                write!(f, "Invalid argument: {}", err),
            NotOperation(ref name) =>
                write!(f, "Not operation: {}", name),
            TooFewArguments =>
                write!(f, "Too few arguments"),
        }
    }
}

impl error::Error for ParsingError {
    fn description(&self) -> &str {
        "Parsing error"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl From<ParsingError> for ChryError {
    fn from(error: ParsingError) -> Self {
        ChryError::Parse(s!(error))
    }
}


impl EventName {
    pub fn operation_with_context(&self, context: HashMap<String, String>) -> Operation {
        Operation::AppEvent(self.clone(), context)
    }

    pub fn operation(&self) -> Operation {
        self.operation_with_context(HashMap::new())
    }
}


impl fmt::Debug for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::Operation::*;

        let s = match *self {
            AppEvent(ref ev, _) => return write!(f, "AppEvent({:?})", ev),
            ChangeDirectory(_) => "ChangeDirectory",
            Cherenkov(_) => "Cherenkov",
            Clear => "Clear ",
            Clip(_) => "Clip",
            Context(_, _) => "Context",
            Controller(_) => "Controller",
            Count(_) => "Count",
            CountDigit(_) => "CountDigit",
            DefineUserSwitch(_, _) => "DefineUserSwitch",
            Delete(_) => "delete",
            Draw => "Draw ",
            Editor(_, _, _) => "Editor",
            Error(ref error) => return write!(f, "Error({:?})", error),
            Expand(_, _) => "Expand",
            First(_, _, _, _) => "First",
            FileChanged(_) => "FileChanged",
            Fill(_, _, _, _, _) => "Fill",
            Filter(_, _) => "Filter",
            FlyLeaves(_) => "FlyLeaves",
            Go(_) => "Go",
            InitialProcess(_, _, _) => "InitialProcess",
            Input(_) => "Input",
            Jump(_) => "Jump",
            KillTimer(_) => "KillTimer",
            Last(_, _, _, _) => "Last",
            LazyDraw(_, _) => "LazyDraw",
            Load(_, _) => "Load",
            LoadDefault => "LoadDefault ",
            MakeVisibles(_) => "MakeVisibles",
            Map(_, _, _) => "Map",
            Mark(_, _) => "Mark",
            Meow => "Meow",
            Message(_) => "Message",
            MoveAgain(_, _, _, _) => "MoveAgain",
            Multi(_, _) => "Multi",
            Next(_, _, _, _) => "Next",
            Nop => "Nop ",
            OperateFile(_) => "OperateFile",
            OperationEntry(_) => "OperationEntry",
            Page(_) => "Page",
            PdfIndex(_, _, _, _, _, _) => "PdfIndex",
            PreFetch(_) => "PreFetch",
            Previous(_, _, _, _) => "Previous",
            Pull => "Pull ",
            Push(_, _, _) => "Push",
            PushArchive(_, _, _) => "PushArchive",
            PushDirectory(_, _, _) => "PushDirectory",
            PushImage(_, _, _, _) => "PushImage",
            PushMemory(_) => "PushMemory",
            PushPdf(_, _, _) => "PushPdf",
            PushSibling(_, _, _, _) => "PushSibling",
            PushURL(_, _, _, _) => "PushURL",
            Query(_, _) => "Query",
            Random => "Random ",
            Refresh(_) => "Refresh",
            ResetImage => "ResetImage ",
            ResetScrolls(_) => "ResetScrolls",
            Save(_, _) => "Save",
            SearchText(_, _, _) => "SearchText",
            Scroll(_, _, _, _) => "Scroll",
            SetEnv(_, _) => "SetEnv",
            Shell(_, _, _, _, _) => "Shell",
            ShellFilter(_, _) => "ShellFilter",
            Show(_, _, _, _) => "Show",
            Shuffle(_) => "Shuffle",
            Sort(_, _, _) => "Sort ",
            TellRegion(_, _, _, _, _) => "TellRegion",
            Timer(_, _, _, _) => "Timer",
            Unclip => "Unclip ",
            Undo(_) => "Undo",
            Unmap(_) => "Unmap",
            Unmark(_) => "Unmark",
            Update(_) => "Update",
            UpdateOption(ref name, _) => return write!(f, "UpdateOption({:?})", name),
            UpdateUI => "UpdateUI ",
            User(_) => "User",
            Views(_, _) => "Views",
            ViewsFellow(_) => "ViewsFellow",
            When(_, _, _) => "When",
            WithMessage(_, _) => "WithMessage",
            Write(_, _) => "Write",
        };
        write!(f, "{}", s)
    }
}
