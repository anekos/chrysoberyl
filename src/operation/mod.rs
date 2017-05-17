
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use cmdline_parser::Parser;

use archive::ArchiveEntry;
use color::Color;
use config::ConfigSource;
use entry::Meta;
use entry;
use filer;
use gui::Direction;
use mapping::{self, mouse_mapping};
use shellexpand_wrapper as sh;
use size::Region;

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
    Draw,
    Editor(Option<String>, Vec<ConfigSource>),
    Expand(bool, Option<PathBuf>), /* recursive, base */
    First(Option<usize>, bool, MoveBy),
    ForceFlush,
    Fragile(PathBuf),
    Initialized,
    Input(mapping::Input),
    Last(Option<usize>, bool, MoveBy),
    LazyDraw(u64, bool), /* serial, to_end */
    LoadConfig(ConfigSource),
    Map(MappingTarget, Vec<String>),
    Multi(Vec<Operation>),
    Next(Option<usize>, bool, MoveBy),
    Nop,
    OperateFile(filer::FileOperation),
    PreFetch(u64),
    Previous(Option<usize>, bool, MoveBy),
    PrintEntries,
    Push(String, Meta),
    PushArchiveEntry(PathBuf, ArchiveEntry),
    PushFile(PathBuf, Meta),
    PushHttpCache(PathBuf, String, Meta),
    PushPdf(PathBuf, Meta),
    PushURL(String, Meta),
    Quit,
    Random,
    Refresh,
    Save(PathBuf, Option<usize>),
    Scroll(Direction, Vec<String>, f64), /* direction, operation, scroll_size_ratio */
    SetEnv(String, Option<String>),
    Shell(bool, bool, Vec<String>), /* async, operation, command_line */
    Show(entry::SearchKey),
    Shuffle(bool), /* Fix current */
    Sort,
    UpdateOption(OptionName, OptionUpdater),
    User(Vec<(String, String)>),
    Unclip,
    Views(Option<usize>, Option<usize>),
    ViewsFellow(bool), /* for_rows */
    WindowResized,
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
    Mouse(u32, Option<mouse_mapping::Area>)
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

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum OptionName {
    AutoPaging,
    CenterAlignment,
    FitTo,
    Reverse,
    Scaling,
    StatusBar,
    StatusFormat,
    PreFetchEnabled,
    PreFetchLimit,
    PreFetchPageSize,
    VerticalViews,
    HorizontalViews,
    ColorWindowBackground,
    ColorStatusBar,
    ColorStatusBarBackground,
    ColorError,
    ColorErrorBackground,
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
            "pre-render"                           => Ok(PreFetchEnabled),
            "pre-render-limit"                     => Ok(PreFetchLimit),
            "pre-render-pages"                     => Ok(PreFetchPageSize),
            "vertical-views"                       => Ok(VerticalViews),
            "horizontal-views"                     => Ok(HorizontalViews),
            "color-window-background"              => Ok(ColorWindowBackground),
            "color-status-bar"                     => Ok(ColorStatusBar),
            "color-status-bar-background"          => Ok(ColorStatusBarBackground),
            "color-error"                          => Ok(ColorError),
            "color-error-background"               => Ok(ColorErrorBackground),
            _ => Err(ParsingError::InvalidOperation(format!("Invalid option name: {}", src)))
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
            Err(ParsingError::NotOperation) => Ok(Operation::Push(o!(s), entry::new_empty_meta())),
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
            "@cherenkov"                 => parse_cherenkov(whole),
            "@clear"                     => Ok(Clear),
            "@copy"                      => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Copy(path, if_exist))),
            "@count"                     => parse_count(whole),
            "@cycle"                     => parse_option_cycle(whole),
            "@disable"                   => parse_option_1(whole, OptionUpdater::Disable),
            "@draw"                      => Ok(Draw),
            "@editor"                    => parse_editor(whole),
            "@enable"                    => parse_option_1(whole, OptionUpdater::Enable),
            "@entries"                   => Ok(PrintEntries),
            "@expand"                    => parse_expand(whole),
            "@first" | "@f"              => parse_move(whole, First),
            "@force-flush"               => Ok(ForceFlush),
            "@fragile"                   => parse_command1(whole, |it| Fragile(sh::expand_to_pathbuf(&it))),
            "@input"                     => parse_input(whole),
            "@last" | "@l"               => parse_move(whole, Last),
            "@load"                      => parse_load(whole),
            "@map"                       => parse_map(whole),
            "@multi"                     => parse_multi(whole),
            "@move"                      => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Move(path, if_exist))),
            "@next" | "@n"               => parse_move(whole, Next),
            "@prev" | "@p" | "@previous" => parse_move(whole, Previous),
            "@push"                      => parse_push(whole, |it, meta| Push(sh::expand(&it), meta)),
            "@push-pdf"                  => parse_push(whole, |it, meta| PushPdf(sh::expand_to_pathbuf(&it), meta)),
            "@push-file"                 => parse_push(whole, |it, meta| PushFile(sh::expand_to_pathbuf(&it), meta)),
            "@push-url"                  => parse_push(whole, PushURL),
            "@quit"                      => Ok(Quit),
            "@random" | "@rand"          => Ok(Random),
            "@refresh" | "@r"            => Ok(Refresh),
            "@save"                      => parse_save(whole),
            "@set"                       => parse_option_set(whole),
            "@set-env"                   => parse_set_env(whole),
            "@scroll"                    => parse_scroll(whole),
            "@shell"                     => parse_shell(whole),
            "@show"                      => parse_show(whole),
            "@shuffle"                   => Ok(Shuffle(false)),
            "@sort"                      => Ok(Sort),
            "@toggle"                    => parse_option_1(whole, OptionUpdater::Toggle),
            "@unclip"                    => Ok(Unclip),
            "@unset"                     => parse_option_1(whole, OptionUpdater::Unset),
            "@user"                      => Ok(Operation::user(args.to_vec())),
            "@views"                     => parse_views(whole),
            ";"                          => parse_multi_args(args, ";"),
            _ => Err(format!("Unknown operation: {}", name))
        } .map_err(ParsingError::InvalidOperation)
    } else {
        Ok(Nop)
    }
}




#[cfg(test)]#[test]
fn test_parse() {
    use std::sync::Arc;
    use std::path::Path;
    use self::Operation::*;
    use mapping::mouse_mapping::Area;

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
    assert_eq!(p("@First"), First(None, false, MoveBy::Page));
    assert_eq!(p("@Next"), Next(None, false, MoveBy::Page));
    assert_eq!(p("@Previous"), Previous(None, false, MoveBy::Page));
    assert_eq!(p("@Prev"), Previous(None, false, MoveBy::Page));
    assert_eq!(p("@Last"), Last(None, false, MoveBy::Page));
    assert_eq!(p("@First 1"), First(Some(1), false, MoveBy::Page));
    assert_eq!(p("@Next 2"), Next(Some(2), false, MoveBy::Page));
    assert_eq!(p("@Previous 3"), Previous(Some(3), false, MoveBy::Page));
    assert_eq!(p("@Prev 4"), Previous(Some(4), false, MoveBy::Page));
    assert_eq!(p("@Last 5"), Last(Some(5), false, MoveBy::Page));
    assert_eq!(p("@Last -i 5"), Last(Some(5), true, MoveBy::Page));
    assert_eq!(p("@Last --ignore-views 5"), Last(Some(5), true, MoveBy::Page));

    // @push*
    assert_eq!(p("@push http://example.com/moge.jpg"), Push(o!("http://example.com/moge.jpg"), Arc::new(vec![])));
    assert_eq!(p("@push-file /hoge/moge.jpg"), PushFile(pathbuf("/hoge/moge.jpg"), Arc::new(vec![])));
    assert_eq!(p("@push-url http://example.com/moge.jpg"), PushURL(o!("http://example.com/moge.jpg"), Arc::new(vec![])));

    // @map
    assert_eq!(q("@map key k @first"), Ok(Map(MappingTarget::Key(vec![s!("k")]), vec![o!("@first")])));
    assert_eq!(p("@map k k @next"), Map(MappingTarget::Key(vec![s!("k")]), vec![o!("@next")]));
    assert_eq!(p("@map key k @next"), Map(MappingTarget::Key(vec![s!("k")]), vec![o!("@next")]));
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
    assert_eq!(p("; @first ; @next"), Multi(vec![First(None, false, MoveBy::Page), Next(None, false, MoveBy::Page)]));
    assert_eq!(p("@multi / @first / @next"), Multi(vec![First(None, false, MoveBy::Page), Next(None, false, MoveBy::Page)]));

    // Shell
    assert_eq!(p("@shell ls -l -a"), Shell(true, false, vec![o!("ls"), o!("-l"), o!("-a")]));
    assert_eq!(p("@shell --async ls -l -a"), Shell(true, false, vec![o!("ls"), o!("-l"), o!("-a")]));
    assert_eq!(p("@shell --async --operation ls -l -a"), Shell(true, true, vec![o!("ls"), o!("-l"), o!("-a")]));
    assert_eq!(p("@shell --sync ls -l -a"), Shell(false, false, vec![o!("ls"), o!("-l"), o!("-a")]));

    // Invalid command
    assert_eq!(p("Meow Meow"), Push(o!("Meow Meow"), Arc::new(vec![])));
    assert_eq!(p("expand /foo/bar.txt"), Push(o!("expand /foo/bar.txt"), Arc::new(vec![])));

    // Shell quotes
    assert_eq!(
        p(r#"@Push "http://example.com/sample.png""#),
        Push(o!("http://example.com/sample.png"), Arc::new(vec![])));

    // Shell quotes
    assert_eq!(
        p(r#"@Push 'http://example.com/sample.png'"#),
        Push(o!("http://example.com/sample.png"), Arc::new(vec![])));

    // Ignore case
    assert_eq!(p("@ShuFFle"), Shuffle(false));
}
