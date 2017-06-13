
use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use cmdline_parser::Parser;

use archive::ArchiveEntry;
use color::Color;
use entry::Meta;
use entry;
use expandable::Expandable;
use filer;
use gui::Direction;
use mapping::{self, mouse_mapping};
use shellexpand_wrapper as sh;
use size::Region;
use session::Session;

mod parser;



#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Cherenkov(CherenkovParameter),
    CherenkovClear,
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
    Fill(Region, usize), /* region, cell index */
    Fragile(PathBuf),
    Initialized,
    Input(mapping::Input),
    KillTimer(String),
    Last(Option<usize>, bool, MoveBy, bool),
    LazyDraw(u64, bool), /* serial, to_end */
    Load(PathBuf),
    LoadDefault,
    Map(MappingTarget, Vec<String>),
    MoveEntry(entry::Position, entry::Position),
    Multi(VecDeque<Operation>, bool), /* operations, async */
    Next(Option<usize>, bool, MoveBy, bool),
    Nop,
    OperateFile(filer::FileOperation),
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
    Save(Option<PathBuf>, Vec<Session>),
    Scroll(Direction, Vec<String>, f64), /* direction, operation, scroll_size_ratio */
    SetEnv(String, Option<Expandable>),
    Shell(bool, bool, Vec<Expandable>, Vec<Session>), /* async, operation, command_line, session */
    ShellFilter(Vec<Expandable>),
    Show(entry::SearchKey),
    Shuffle(bool), /* Fix current */
    Sort,
    TellRegion(Region),
    Timer(String, Vec<String>, Duration, Option<usize>),
    UpdateOption(OptionName, OptionUpdater),
    User(Vec<(String, String)>),
    Unclip,
    Views(Option<usize>, Option<usize>),
    ViewsFellow(bool), /* for_rows */
    WindowResized,
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
pub enum OperationContext {
    Input(mapping::Input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum MappingTarget {
    Key(Vec<String>),
    Mouse(u32, Option<mouse_mapping::Area>),
    Event(String, Option<String>),
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
    AutoPaging,
    CenterAlignment,
    FitTo,
    Reverse,
    Scaling,
    StatusBar,
    StatusFormat,
    TitleFormat,
    PreFetchEnabled,
    PreFetchLimit,
    PreFetchPageSize,
    VerticalViews,
    HorizontalViews,
    RegionFunction,
    ColorWindowBackground,
    ColorStatusBar,
    ColorStatusBarBackground,
    ColorError,
    ColorErrorBackground,
    ColorFill,
    User(String),
}

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

impl FromStr for OptionName {
    type Err = ParsingError;

    fn from_str(src: &str) -> Result<Self, ParsingError> {
        use self::OptionName::*;

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
            "region-function" | "region-func"      => Ok(RegionFunction),
            "window-background-color"              => Ok(ColorWindowBackground),
            "status-bar-color"                     => Ok(ColorStatusBar),
            "status-bar-background-color"          => Ok(ColorStatusBarBackground),
            "error-color"                          => Ok(ColorError),
            "error-background-color"               => Ok(ColorErrorBackground),
            "fill-color"                           => Ok(ColorFill),
            user => Ok(User(o!(user)))
        }
    }
}

impl Default for OptionName {
    fn default() -> Self {
        OptionName::StatusBar
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
            "@copy"                         => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Copy(path, if_exist))),
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
            "@first" | "@f"                 => parse_move(whole, First),
            "@fragile"                      => parse_command1(whole, |it| Fragile(sh::expand_to_pathbuf(&it))),
            "@input"                        => parse_input(whole),
            "@kill-timer"                   => parse_kill_timer(whole),
            "@last" | "@l"                  => parse_move(whole, Last),
            "@load"                         => parse_load(whole),
            "@map"                          => parse_map(whole),
            "@move"                         => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Move(path, if_exist))),
            "@move-entry"                   => parse_move_entry(whole),
            "@multi"                        => parse_multi(whole),
            "@next" | "@n"                  => parse_move(whole, Next),
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
            "@save"                         => parse_save(whole),
            "@scroll"                       => parse_scroll(whole),
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




#[cfg(test)]#[test]
fn test_parse() {
    use std::path::Path;
    use self::Operation::*;
    use mapping::mouse_mapping::Area;

    macro_rules! vecs {
        ($($args:expr),*) => {
            vec![$(s!($args)),*]
        }
    }

    fn p(s: &str) -> Operation {
        Operation::parse_fuzziness(s).unwrap()
    }

    fn q(s: &str) -> Result<Operation, ParsingError> {
        Operation::parse(s)
    }

    fn pathbuf(s: &str) -> PathBuf {
        Path::new(s).to_path_buf()
    }

    // Simple
    assert_eq!(p("@shuffle"), Shuffle(false));
    assert_eq!(p("@entries"), PrintEntries);
    assert_eq!(p("@refresh"), Refresh);
    assert_eq!(p("@sort"), Sort);
    assert_eq!(p("@editor"), Editor(None, vec![]));

    // Move
    assert_eq!(p("@First"), First(None, false, MoveBy::Page, false));
    assert_eq!(p("@Next"), Next(None, false, MoveBy::Page, false));
    assert_eq!(p("@Previous"), Previous(None, false, MoveBy::Page, false));
    assert_eq!(p("@Prev"), Previous(None, false, MoveBy::Page, false));
    assert_eq!(p("@Last"), Last(None, false, MoveBy::Page, false));
    assert_eq!(p("@First 1"), First(Some(1), false, MoveBy::Page, false));
    assert_eq!(p("@Next 2"), Next(Some(2), false, MoveBy::Page, false));
    assert_eq!(p("@Previous 3"), Previous(Some(3), false, MoveBy::Page, false));
    assert_eq!(p("@Prev 4"), Previous(Some(4), false, MoveBy::Page, false));
    assert_eq!(p("@Last 5"), Last(Some(5), false, MoveBy::Page, false));
    assert_eq!(p("@Last -i 5"), Last(Some(5), true, MoveBy::Page, false));
    assert_eq!(p("@Last --ignore-views 5"), Last(Some(5), true, MoveBy::Page, false));
    assert_eq!(p("@Last --ignore-views --archive 5"), Last(Some(5), true, MoveBy::Archive, false));

    // @push*
    assert_eq!(p("@push http://example.com/moge.jpg"), Push(o!("http://example.com/moge.jpg"), None));
    assert_eq!(p("@push-image /hoge/moge.jpg"), PushImage(pathbuf("/hoge/moge.jpg"), None));
    assert_eq!(p("@push-url http://example.com/moge.jpg"), PushURL(o!("http://example.com/moge.jpg"), None));

    // @map
    assert_eq!(q("@map key k @first"), Ok(Map(MappingTarget::Key(vecs!["k"]), vec![o!("@first")])));
    assert_eq!(p("@map k k @next"), Map(MappingTarget::Key(vecs!["k"]), vec![o!("@next")]));
    assert_eq!(p("@map key k @next"), Map(MappingTarget::Key(vecs!["k"]), vec![o!("@next")]));
    assert_eq!(q("@map mouse 6 @last"), Ok(Map(MappingTarget::Mouse(6, None), vec![o!("@last")])));
    assert_eq!(p("@map m 6 @last"), Map(MappingTarget::Mouse(6, None), vec![o!("@last")]));
    assert_eq!(p("@map m --area 0.1x0.2-0.3x0.4 6 @last"), Map(MappingTarget::Mouse(6, Some(Area::new(0.1, 0.2, 0.3, 0.4))), vec![o!("@last")]));

    // Expand
    assert_eq!(p("@expand /foo/bar.txt"), Expand(false, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(p("@expand"), Expand(false, None));
    assert_eq!(p("@expand --recursive /foo/bar.txt"), Expand(true, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(p("@expand --recursive"), Expand(true, None));

    // Option
    assert_eq!(p("@toggle status"), UpdateOption(OptionName::StatusBar, OptionUpdater::Toggle));
    assert_eq!(p("@toggle status-bar"), UpdateOption(OptionName::StatusBar, OptionUpdater::Toggle));
    assert_eq!(p("@enable center"), UpdateOption(OptionName::CenterAlignment, OptionUpdater::Enable));
    assert_eq!(p("@disable center-alignment"), UpdateOption(OptionName::CenterAlignment, OptionUpdater::Disable));
    assert_eq!(p("@disable fit"), UpdateOption(OptionName::FitTo, OptionUpdater::Disable));

    // Multi
    assert_eq!(p("; @first ; @next"), Multi(VecDeque::from(vec![First(None, false, MoveBy::Page, false), Next(None, false, MoveBy::Page, false)]), true));
    assert_eq!(p("@multi / @first / @next"), Multi(VecDeque::from(vec![First(None, false, MoveBy::Page, false), Next(None, false, MoveBy::Page, false)]), true));

    // Shell
    assert_eq!(p("@shell ls -l -a"), Shell(true, false, vec![o!("ls"), o!("-l"), o!("-a")], vec![]));
    assert_eq!(p("@shell --async ls -l -a"), Shell(true, false, vec![o!("ls"), o!("-l"), o!("-a")], vec![]));
    assert_eq!(p("@shell --async --operation ls -l -a"), Shell(true, true, vec![o!("ls"), o!("-l"), o!("-a")], vec![]));
    assert_eq!(p("@shell --sync ls -l -a"), Shell(false, false, vec![o!("ls"), o!("-l"), o!("-a")], vec![]));

    // Invalid command
    assert_eq!(p("Meow Meow"), Push(o!("Meow Meow"), None));
    assert_eq!(p("expand /foo/bar.txt"), Push(o!("expand /foo/bar.txt"), None));

    // Shell quotes
    assert_eq!(
        p(r#"@Push "http://example.com/sample.png""#),
        Push(o!("http://example.com/sample.png"), None));

    // Shell quotes
    assert_eq!(
        p(r#"@Push 'http://example.com/sample.png'"#),
        Push(o!("http://example.com/sample.png"), None));

    // Ignore case
    assert_eq!(p("@ShuFFle"), Shuffle(false));
}
