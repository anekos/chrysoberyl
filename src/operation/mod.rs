
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use cmdline_parser::Parser;

use crate::archive::ArchiveEntry;
use crate::chainer;
use crate::cherenkov::Operator;
use crate::cherenkov::fill::Shape;
use crate::cherenkov::nova::Seed;
use crate::color::Color;
use crate::command_line;
use crate::controller;
use crate::entry::filter::expression::Expr as FilterExpr;
use crate::entry::{Meta, EntryType};
use crate::entry;
use crate::errors::{AppResult, AppError, ParsingError};
use crate::events::EventName;
use crate::expandable::Expandable;
use crate::filer;
use crate::gui::Direction;
use crate::key::Key;
use crate::key::KeySequence;
use crate::mapping;
use crate::poppler;
use crate::session::Session;
use crate::size::Region;

mod parser;
pub mod option;

use self::option::{OptionName, OptionUpdater};



#[derive(Clone)]
pub enum Operation {
    Apng(PathBuf, u8), /* path, length */
    AppEvent(EventName, HashMap<String, String>),
    Backward,
    Chain(chainer::Target),
    ChangeDirectory(Expandable),
    Cherenkov(CherenkovParameter),
    CherenkovReset,
    Clear,
    ClearCacheEntry(entry::Key), /* internal use only */
    Clip(Region),
    Context(OperationContext, Box<Operation>),
    Controller(controller::Source),
    CopyToClipboard(ClipboardSelection),
    Count(Option<usize>),
    CountDigit(u8),
    DefineUserSwitch(String, Vec<Vec<String>>),
    Delete(Box<entry::filter::expression::Expr>),
    DetectEyes(CherenkovParameter),
    Draw,
    Editor(Vec<Expandable>, Vec<Expandable>, Vec<Session>, bool, bool), /* editor_command, options, session, comment_out, freeze */
    Error(String),
    Eval(Vec<String>),
    Expand(bool, Option<PathBuf>), /* recursive, base */
    FileChanged(PathBuf),
    Fill(Shape, Option<Region>, Color, Option<Operator>, bool, usize), /* shape, region, color, fill_operator, mask, cell index */
    Filter(bool, Box<Option<entry::filter::expression::Expr>>), /* dynamic, filter expression */
    Fire(mapping::Mapped),
    First(Option<usize>, bool, MoveBy, bool), /* count, ignore-views, archive/page, wrap */
    FlushBuffer,
    FlyLeaves(usize),
    Forward,
    Gif(PathBuf, u8, bool), /* path, length, show */
    Go(entry::SearchKey),
    InitialProcess(Vec<command_line::Entry>, bool, bool), /* command_lin::entries, shuffle, stdin_as_binary */
    Input(Vec<mapping::Mapped>),
    Jump(String, bool), /* marker name, load */
    KillTimer(String),
    Last(Option<usize>, bool, MoveBy, bool),
    LazyDraw(u64, bool), /* serial, to_end */
    LinkAction(Vec<String>),
    Load(Expandable, bool), /* path, search_path */
    LoadDefault,
    LoadUI(Expandable, bool),
    MakeVisibles(Vec<Option<Region>>),
    Map(MappingTarget, Option<usize>, Vec<String>), /* target, remain, operation */
    Mark(String, Option<(String, usize, Option<EntryType>)>),
    Meow,
    Message(Option<String>, bool),
    MoveAgain(Option<usize>, bool, MoveBy, bool, bool), /* count, ignore-views, archive/page, wrap, reverse */
    Multi(VecDeque<Operation>, bool), /* operations, async */
    Next(Option<usize>, bool, MoveBy, bool, bool), /* count, ignore_views, move_by, wrap, forget */
    Nop,
    OperateFile(filer::FileOperation),
    Page(usize),
    PdfIndex(bool, bool, bool, Vec<Expandable>, poppler::index::Format, Option<String>), /* async, read_operations, search_path, ... */
    Pointer(bool),
    PopCount,
    PreFetch(u64),
    Previous(Option<usize>, bool, MoveBy, bool, bool), /* count, ignore_views, move_by, wrap, forget */
    Pull,
    Push(Expandable, Option<Meta>, bool, bool), /* path, meta, force, show */
    PushArchive(Expandable, Option<Meta>, bool, bool), /* path, meta, force, show */
    PushClipboard(ClipboardSelection, bool, Option<Meta>, bool, bool), /* selection, as_operation, meta, force, show */
    PushCount,
    PushDirectory(Expandable, Option<Meta>, bool), /* path, meta, force */
    PushImage(Expandable, Option<Meta>, bool, bool, Option<u8>), /* path, meta, force, show, expand-level */
    PushMemory(Vec<u8>, Option<Meta>, bool), /* memory, meta, show */
    PushMessage(String, Option<Meta>, bool), /* message, meta, show */
    PushPdf(Expandable, Option<Meta>, bool, bool), /* path, meta, force, show */
    PushSibling(bool, bool, Option<Meta>, bool, bool), /* next?, clear, meta, force, show */
    PushURL(String, Option<Meta>, bool, bool, Option<EntryType>), /* path, meta, force, show, entry_type */
    Query(Vec<String>, Option<String>), /* operation, caption */
    Queue(Vec<String>, usize),
    Random,
    Record(usize, usize, entry::Key), /* minimum_move, index, key */
    RecordPre(Vec<String>, usize),
    Refresh(bool), /* image_cache */
    RemoveEffects,
    ResetFocus,
    UIAction(UIActionType),
    ResetScrolls(bool), /* to_end */
    Save(PathBuf, Vec<Session>, bool), /* path, sessions, freeze */
    SearchText(Option<String>, bool, Color), /* text, backward */
    Scroll(Direction, f64, bool, bool, Vec<String>, Option<Direction>), /* direction, scroll_size_ratio, crush, reset_at_end, operation, reset_scrolls_1 */
    SetEnv(String, Option<Expandable>),
    Shell(bool, ReadAs, bool, Vec<Expandable>, Vec<Session>, bool), /* async, operation, search_path, command_line, session, freeze */
    ShellFilter(Vec<Expandable>, bool), /* path, search_path */
    Show(Option<usize>, bool, MoveBy, bool), /* count, ignore-views, archive/page, wrap */
    ShowCommandLine(String),
    Shuffle(bool), /* Fix current */
    Sort(bool, SortKey, bool), /* fix_current, key, reverse */
    Sorter(bool, Vec<Expandable>, bool), /* fix_current, command, reverse */
    TellRegion(f64, f64, f64, f64, Key), /* lef,t top, right, bottom, mousesbutton */
    Timer(Option<String>, Vec<String>, Duration, Option<usize>, bool),
    Unchain(chainer::Target),
    Unclip,
    Undo(Option<usize>),
    Unmap(MappingTarget),
    Unmark(Option<String>), /* all or given */
    Update(Updated),
    UpdateOption(OptionName, OptionUpdater),
    UpdateUI,
    User(Vec<(String, String)>),
    Views(Option<usize>, Option<usize>, bool), /* cols, rows, ignore_views */
    ViewsFellow(bool, bool), /* for_rows, ignore_views */
    WakeupTimer(String),
    When(FilterExpr, bool, Vec<String>), /* filter, reverse(unless), operation */
    WithMessage(Option<String>, Box<Operation>),
    Write(PathBuf, Option<usize>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CherenkovParameter {
    pub color: Color,
    pub n_spokes: usize,
    pub radius: f64,
    pub random_hue: f64,
    pub seed: Seed,
    pub threads: Option<u8>,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OperationContext {
    pub cell_index: Option<usize>,
    pub mapped: mapping::Mapped,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MappingTarget {
    Operation(String),
    Input(KeySequence, Option<Region>),
    Event(Option<EventName>, Option<String>),
    Region(Key),
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum MoveBy {
    Page,
    Archive,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ReadAs {
    Binary,
    Paths,
    Ignore,
    Operations,
}


#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum QueuedOperation {
    PushArchive(PathBuf, Option<Meta>, bool, bool, Option<String>), /* path, meta, force, show, remote-url */
    PushArchiveEntry(PathBuf, ArchiveEntry, Option<Meta>, bool, bool, Option<String>), /* path, archive-entry, meta, force, show, remote-url */
    PushDirectory(PathBuf, Option<Meta>, bool), /* path, meta, force */
    PushMessage(String, Option<Meta>, bool), /* message, meta, show */
    PushImage(PathBuf, Option<Meta>, bool, bool, Option<u8>, Option<String>), /* path, meta, force, show, expand-level, remote-url */
    PushMemory(Vec<u8>, Option<Meta>, bool), /* memory */
    PushPdf(PathBuf, Option<Meta>, bool, bool, Option<String>), /* path, meta, force, show, remote-url */
    PushPdfEntries(PathBuf, usize, Option<Meta>, bool, bool, Option<String>), /* path, pages, meta, force, show, remote-url */
}

#[derive(Default, Debug, Clone)]
pub struct Updated {
    pub counter: bool,
    pub image: bool,
    pub image_options: bool,
    pub label: bool,
    pub message: bool,
    pub pointer: bool,
    pub remote: bool,
    pub size: bool,
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
pub enum UIActionType {
    SendOperation,
    Close,
}

#[derive(Clone, Copy)]
pub enum ClipboardSelection {
    Clipboard,
    Primary,
    Secondary,
}


impl FromStr for Operation {
    type Err = AppError;

    fn from_str(src: &str) -> AppResult<Self> {
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
            "@apng"                         => parse_apng(whole),
            "@backward" | "@back"           => Ok(Backward),
            "@cd" | "@chdir" | "@change-directory"
                                            => parse_command1(whole, Operation::ChangeDirectory),
            "@chain"                        => parse_chainer(whole, Operation::Chain),
            "@cherenkov"                    => parse_cherenkov(whole),
            "@cherenkov-reset"              => Ok(CherenkovReset),
            "@clear"                        => Ok(Clear),
            "@clip"                         => parse_clip(whole),
            "@command-line"                 => parse_command1(whole, Operation::ShowCommandLine),
            "@controller-fifo" | "@control-fifo"
                                            => parse_controller(whole, controller::Source::Fifo),
            "@controller-file" | "@control-file"
                                            => parse_controller(whole, controller::Source::File),
            "@controller-socket" | "@control-socket"
                                            => parse_controller_socket(whole),
            "@copy-to-clipboard" | "@clipboard"
                                            => parse_copy_to_clipboard(whole),
            "@count"                        => parse_count(whole),
            "@cycle"                        => parse_option_cycle(whole),
            "@dec" | "@decrement" | "@decrease" | "@--"
                                            => parse_usize(whole, OptionUpdater::Decrement, 1),
            "@default"                      => Ok(LoadDefault),
            "@define-switch"                => parse_define_switch(whole),
            "@delete"                       => parse_delete(whole),
            "@disable"                      => parse_option_1(whole, OptionUpdater::Disable),
            "@draw"                         => Ok(Draw),
            "@editor"                       => parse_editor(whole),
            "@enable"                       => parse_option_1(whole, OptionUpdater::Enable),
            "@eval"                         => Ok(Operation::Eval(whole[1..].to_vec())),
            "@expand"                       => parse_expand(whole),
            "@file-copy"                    => parse_file(whole, filer::FileOperation::new_copy),
            "@file-move"                    => parse_file(whole, filer::FileOperation::new_move),
            "@fill"                         => parse_fill(whole),
            "@filter"                       => parse_filter(whole),
            "@first" | "@f"                 => parse_move(whole, First),
            "@fire"                         => parse_fire(whole),
            "@flush-buffer"                 => Ok(FlushBuffer),
            "@fly-leaves"                   => parse_fly_leaves(whole),
            "@forward" | "@fwd"             => Ok(Forward),
            "@gif"                          => parse_gif(whole),
            "@go"                           => parse_go(whole),
            "@inc" | "@increment" | "@increase" | "@++"
                                            => parse_usize(whole, OptionUpdater::Increment, 1),
            "@input"                        => parse_input(whole),
            "@jump"                         => parse_jump(whole),
            "@kill-timer"                   => parse_kill_timer(whole),
            "@last" | "@l"                  => parse_move(whole, Last),
            "@load"                         => parse_load(whole, Load),
            "@load-ui"                      => parse_load(whole, LoadUI),
            "@map"                          => parse_map(whole, true),
            "@mark"                         => parse_mark(whole),
            "@meow"                         => Ok(Meow),
            "@message"                      => parse_message(whole),
            "@move-again"                   => parse_move_again(whole),
            "@multi"                        => parse_multi(whole),
            "@next" | "@n"                  => parse_move5(whole, Next),
            "@nop"                          => Ok(Nop),
            "@page"                         => parse_page(whole),
            "@pdf-index"                    => parse_pdf_index(whole),
            "@link-action" | "@link"        => Ok(Operation::LinkAction(whole[1..].to_vec())),
            "@prev" | "@p" | "@previous"    => parse_move5(whole, Previous),
            "@pop-count"                    => Ok(PopCount),
            "@push"                         => parse_push(whole, |it, meta, force, show| Push(Expandable::new(it), meta, force, show)),
            "@push-count"                   => Ok(PushCount),
            "@push-archive"                 => parse_push(whole, |it, meta, force, show| PushArchive(Expandable::new(it), meta, force, show)),
            "@push-clipboard"               => parse_push_clipboard(whole),
            "@push-directory" | "@push-dir" => parse_push(whole, |it, meta, force, _| PushDirectory(Expandable::new(it), meta, force)),
            "@push-image"                   => parse_push_image(whole),
            "@push-message"                 => parse_push_message(whole),
            "@push-next"                    => parse_push_sibling(whole, true),
            "@push-pdf"                     => parse_push(whole, |it, meta, force, show| PushPdf(Expandable::new(it), meta, force, show)),
            "@push-previous" | "@push-prev" => parse_push_sibling(whole, false),
            "@push-url"                     => parse_push_url(whole),
            "@query"                        => parse_query(whole),
            "@queue"                        => parse_queue(whole),
            "@quit"                         => Ok(EventName::Quit.operation()),
            "@record"                       => parse_record_pre(whole),
            "@random" | "@rand"             => Ok(Random),
            "@refresh" | "@r"               => parse_refresh(whole),
            "@remove-effects"               => Ok(RemoveEffects),
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
            "@unchain"                      => parse_chainer(whole, Operation::Unchain),
            "@unclip"                       => Ok(Unclip),
            "@undo"                         => parse_undo(whole),
            "@unless"                       => parse_when(whole, true),
            "@unmap"                        => parse_map(whole, false),
            "@unmark"                       => parse_command1(whole, |it| Unmark(Some(it))),
            "@unmark-all"                   => Ok(Unmark(None)),
            "@unset"                        => parse_option_1(whole, OptionUpdater::Unset),
            "@update"                       => parse_update(whole),
            "@user"                         => Ok(Operation::user(args)),
            "@views" | "@v"                 => parse_views(whole),
            "@when"                         => parse_when(whole, false),
            "@write"                        => parse_write(whole),
            name => if let Some(stripped) = name.strip_prefix('@') {
                Ok(Operation::Fire(mapping::Mapped::Operation(o!(stripped), whole[1..].to_vec())))
            } else {
                Err(ParsingError::NotOperation(o!(name)))
            }
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
    pub fn parse_from_vec(whole: &[String]) -> AppResult<Operation> {
        Ok(_parse_from_vec(whole)?)
    }

    pub fn parse(s: &str) -> AppResult<Operation> {
        Ok(_parse_from_str(s)?)
    }

    pub fn parse_fuzziness(s: &str) -> AppResult<Operation> {
        match _parse_from_str(s) {
            Err(ParsingError::NotOperation(_)) => Ok(Operation::Push(Expandable::new(o!(s)), None, false, false)),
            Err(err) => Err(err.into()),
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


impl Default for ClipboardSelection {
    fn default() -> Self {
        ClipboardSelection::Clipboard
    }
}

impl EventName {
    pub fn operation_with_env(&self, env: HashMap<String, String>) -> Operation {
        Operation::AppEvent(self.clone(), env)
    }

    pub fn operation(&self) -> Operation {
        self.operation_with_env(HashMap::new())
    }
}


impl fmt::Debug for Operation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        use self::Operation::*;

        let s = match *self {
            Apng(_, _) => "Apng",
            AppEvent(ref ev, _) => return write!(f, "AppEvent({:?})", ev),
            Backward => "Backward",
            Chain(_) => "Chain",
            ChangeDirectory(_) => "ChangeDirectory",
            Cherenkov(_) => "Cherenkov",
            CherenkovReset => "CherenkovReset",
            Clear => "Clear",
            ClearCacheEntry(_) => "ClearCacheEntry",
            Clip(_) => "Clip",
            Context(_, _) => "Context",
            Controller(_) => "Controller",
            CopyToClipboard(_) => "CopyToClipboard",
            Count(_) => "Count",
            CountDigit(_) => "CountDigit",
            DefineUserSwitch(_, _) => "DefineUserSwitch",
            Delete(_) => "delete",
            DetectEyes(_) => "DetectEyes",
            Draw => "Draw ",
            Editor(_, _, _, _, _) => "Editor",
            Error(ref error) => return write!(f, "Error({:?})", error),
            Eval(_) => "Eval",
            Expand(_, _) => "Expand",
            FileChanged(_) => "FileChanged",
            Fill(_, _, _, _, _, _) => "Fill",
            Filter(_, _) => "Filter",
            Fire(_) => "Fire",
            First(_, _, _, _) => "First",
            FlushBuffer => "FlushBuffer",
            FlyLeaves(_) => "FlyLeaves",
            Forward => "Forward",
            Gif(_, _, _) => "Gif",
            Go(_) => "Go",
            InitialProcess(_, _, _) => "InitialProcess",
            Input(_) => "Input",
            Jump(_, _) => "Jump",
            KillTimer(_) => "KillTimer",
            Last(_, _, _, _) => "Last",
            LazyDraw(_, _) => "LazyDraw",
            LinkAction(_) => "LinkAction",
            Load(_, _) => "Load",
            LoadDefault => "LoadDefault ",
            LoadUI(_, _) => "LoadUI",
            MakeVisibles(_) => "MakeVisibles",
            Map(_, _, _) => "Map",
            Mark(_, _) => "Mark",
            Meow => "Meow",
            Message(_, _) => "Message",
            MoveAgain(_, _, _, _, _) => "MoveAgain",
            Multi(_, _) => "Multi",
            Next(_, _, _, _, _) => "Next",
            Nop => "Nop ",
            OperateFile(_) => "OperateFile",
            Page(_) => "Page",
            PdfIndex(_, _, _, _, _, _) => "PdfIndex",
            PreFetch(_) => "PreFetch",
            Previous(_, _, _, _, _) => "Previous",
            Pointer(visibility) => return write!(f, "Pointer({:?})", visibility),
            PopCount => "PopCount",
            Pull => "Pull ",
            Push(_, _, _, _) => "Push",
            PushCount => "PushCount",
            PushArchive(_, _, _, _) => "PushArchive",
            PushClipboard(_, _, _, _, _) => "PushClipboard",
            PushDirectory(_, _, _) => "PushDirectory",
            PushImage(_, _, _, _, _) => "PushImage",
            PushMessage(_, _, _) => "PushMessage",
            PushMemory(_, _, _) => "PushMemory",
            PushPdf(_, _, _, _) => "PushPdf",
            PushSibling(_, _, _, _, _) => "PushSibling",
            PushURL(_, _, _, _, _) => "PushURL",
            Query(_, _) => "Query",
            Queue(_, _) => "Queue",
            Random => "Random ",
            Record(_, _, _) => "Record",
            RecordPre(_, _) => "RecordPre",
            Refresh(_) => "Refresh",
            RemoveEffects => "RemoveEffects ",
            ResetFocus => "ResetFocus",
            ResetScrolls(_) => "ResetScrolls",
            Save(_, _, _) => "Save",
            SearchText(_, _, _) => "SearchText",
            Scroll(_, _, _, _, _, _) => "Scroll",
            SetEnv(_, _) => "SetEnv",
            Shell(_, _, _, _ , _, _) => "Shell",
            ShellFilter(_, _) => "ShellFilter",
            Show(_, _, _, _) => "Show",
            ShowCommandLine(_) => "ShowCommandLine",
            Shuffle(_) => "Shuffle",
            Sort(_, _, _) => "Sort",
            Sorter(_, _, _) => "Sorter",
            TellRegion(_, _, _, _, _) => "TellRegion",
            Timer(_, _, _, _, _) => "Timer",
            UIAction(_) => "UIAction",
            Unchain(_) => "Unchain",
            Unclip => "Unclip ",
            Undo(_) => "Undo",
            Unmap(_) => "Unmap",
            Unmark(_) => "Unmark",
            Update(_) => "Update",
            UpdateOption(ref name, _) => return write!(f, "UpdateOption({:?})", name),
            UpdateUI => "UpdateUI ",
            User(_) => "User",
            Views(_, _, _) => "Views",
            ViewsFellow(_, _) => "ViewsFellow",
            WakeupTimer(_) => "WakeupTimer",
            When(_, _, _) => "When",
            WithMessage(_, _) => "WithMessage",
            Write(_, _) => "Write",
        };
        write!(f, "{}", s)
    }
}
