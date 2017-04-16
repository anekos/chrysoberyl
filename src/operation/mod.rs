
use std::path:: PathBuf;
use std::str::FromStr;

use cmdline_parser::Parser;
use css_color_parser::Color as CssColor;

use archive::ArchiveEntry;
use config::ConfigSource;
use entry::{Meta, new_meta};
use filer;
use gui::ColorTarget;
use mapping::{self, mouse_mapping};
use state::StateName;

mod parser;
mod utils;

use self::utils::*;
use state::ScalingMethod;



#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Cherenkov(CherenkovParameter),
    CherenkovClear,
    Clear,
    Color(ColorTarget, CssColor),
    Context(OperationContext, Box<Operation>),
    Count(Option<usize>),
    CountDigit(u8),
    Editor(Option<String>, Vec<ConfigSource>),
    Expand(bool, Option<PathBuf>), /* recursive, base */
    First(Option<usize>, bool),
    Fragile(PathBuf),
    Input(mapping::Input),
    Last(Option<usize>, bool),
    LazyDraw(u64), /* serial */
    LoadConfig(ConfigSource),
    Map(MappingTarget, Vec<String>),
    Multi(Vec<Operation>),
    Next(Option<usize>, bool),
    Nop,
    OperateFile(filer::FileOperation),
    Previous(Option<usize>, bool),
    PrintEntries,
    Push(String, Meta),
    PushArchiveEntry(PathBuf, ArchiveEntry),
    PushHttpCache(PathBuf, String, Meta),
    PushPdf(PathBuf, Meta),
    PushPath(PathBuf, Meta),
    PushURL(String, Meta),
    Quit,
    Random,
    Refresh,
    Save(PathBuf, Option<usize>),
    ChangeScalingMethod(ScalingMethod),
    Shell(bool, bool, Vec<String>), /* async, operation, command_line */
    Shuffle(bool), /* Fix current */
    Sort,
    UpdateOption(StateName, StateUpdater),
    User(Vec<(String, String)>),
    Views(Option<usize>, Option<usize>),
    ViewsFellow(bool), /* for_rows */
}


#[derive(Clone, Debug, PartialEq)]
pub enum StateUpdater { Toggle, Enable, Disable }

#[derive(Clone, Debug, PartialEq)]
pub struct CherenkovParameter {
    pub radius: f64,
    pub random_hue: f64,
    pub n_spokes: usize,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub color: CssColor,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OperationContext {
    Input(mapping::Input)
}

#[derive(Clone, Debug, PartialEq)]
pub enum MappingTarget {
    Key(String),
    Mouse(u32, Option<mouse_mapping::Area>)
}


impl FromStr for Operation {
    type Err = String;

    fn from_str(src: &str) -> Result<Operation, String> {
        parse(src)
    }
}


impl Operation {
    pub fn from_str_force(s: &str) -> Operation {
        use std::str::FromStr;

        Operation::from_str(s).unwrap_or_else(|_| Operation::Push(expand(s).unwrap_or(s.to_owned()), new_meta(&[])))
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


pub fn parse_from_vec(whole: &[String]) -> Result<Operation, String> {
    use self::Operation::*;
    use filer::FileOperation::{Copy, Move};
    use self::parser::*;

    if let Some(head) = whole.get(0) {
        let name = &*head.to_lowercase();
        let args = &whole[1..];

        if name.starts_with('#') {
            return Ok(Nop)
        }

        match name {
            "@cherenkov"                 => parse_cherenkov(whole),
            "@clear"                     => Ok(Clear),
            "@color"                     => parse_color(whole),
            "@copy"                      => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Copy(path, if_exist))),
            "@count"                     => parse_count(whole),
            "@disable"                   => parse_option_updater(whole, StateUpdater::Disable),
            "@editor"                    => parse_editor(whole),
            "@enable"                    => parse_option_updater(whole, StateUpdater::Enable),
            "@entries"                   => Ok(PrintEntries),
            "@expand"                    => parse_expand(whole),
            "@first" | "@f"              => parse_move(whole, First),
            "@fragile"                   => parse_command1(whole, |it| expand_to_pathbuf(&it).map(Fragile)),
            "@input"                     => parse_input(whole),
            "@last" | "@l"               => parse_move(whole, Last),
            "@load"                      => parse_load(whole),
            "@map"                       => parse_map(whole),
            "@multi"                     => parse_multi(whole),
            "@move"                      => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Move(path, if_exist))),
            "@next" | "@n"               => parse_move(whole, Next),
            "@prev" | "@p" | "@previous" => parse_move(whole, Previous),
            "@push"                      => parse_push(whole, |it, meta| expand(&it).map(|it| Push(it, meta))),
            "@pushpdf"                   => parse_push(whole, |it, meta| expand_to_pathbuf(&it).map(|it| PushPdf(it, meta))),
            "@pushpath"                  => parse_push(whole, |it, meta| expand_to_pathbuf(&it).map(|it| PushPath(it, meta))),
            "@pushurl"                   => parse_push(whole, |it, meta| Ok(PushURL(it, meta))),
            "@quit"                      => Ok(Quit),
            "@random" | "@rand"          => Ok(Random),
            "@refresh" | "@r"            => Ok(Refresh),
            "@save"                      => parse_save(whole),
            "@scaling"                   => parse_scaling(whole),
            "@shell"                     => parse_shell(whole),
            "@shuffle"                   => Ok(Shuffle(false)),
            "@sort"                      => Ok(Sort),
            "@toggle"                    => parse_option_updater(whole, StateUpdater::Toggle),
            "@user"                      => Ok(Operation::user(args.to_vec())),
            "@views"                     => parse_views(whole),
            ";"                          => parse_multi_args(args, ";"),
            _ => Err(format!("Invalid commnad: {}", name))
        }
    } else {
        Ok(Nop)
    }
}

fn parse(s: &str) -> Result<Operation, String> {
    let ps: Vec<String> = Parser::new(s).map(|(_, it)| it).collect();
    parse_from_vec(ps.as_slice())
}



#[cfg(test)]#[test]
fn test_parse() {
    use self::Operation::*;
    use mapping::mouse_mapping::Area;
    use std::sync::Arc;

    fn p(s: &str) -> Operation {
        Operation::from_str_force(s)
    }

    fn q(s: &str) -> Result<Operation, String> {
        s.parse()
    }

    // Simple
    assert_eq!(p("@shuffle"), Shuffle(false));
    assert_eq!(p("@entries"), PrintEntries);
    assert_eq!(p("@refresh"), Refresh);
    assert_eq!(p("@sort"), Sort);
    assert_eq!(p("@editor"), Editor(None, vec![]));

    // Move
    assert_eq!(p("@First"), First(None, false));
    assert_eq!(p("@Next"), Next(None, false));
    assert_eq!(p("@Previous"), Previous(None, false));
    assert_eq!(p("@Prev"), Previous(None, false));
    assert_eq!(p("@Last"), Last(None, false));
    assert_eq!(p("@First 1"), First(Some(1), false));
    assert_eq!(p("@Next 2"), Next(Some(2), false));
    assert_eq!(p("@Previous 3"), Previous(Some(3), false));
    assert_eq!(p("@Prev 4"), Previous(Some(4), false));
    assert_eq!(p("@Last 5"), Last(Some(5), false));
    assert_eq!(p("@Last -i 5"), Last(Some(5), true));
    assert_eq!(p("@Last --ignore-views 5"), Last(Some(5), true));

    // @push*
    assert_eq!(p("@push http://example.com/moge.jpg"), Push(o!("http://example.com/moge.jpg"), Arc::new(vec![])));
    assert_eq!(p("@pushpath /hoge/moge.jpg"), PushPath(pathbuf("/hoge/moge.jpg"), Arc::new(vec![])));
    assert_eq!(p("@pushurl http://example.com/moge.jpg"), PushURL(o!("http://example.com/moge.jpg"), Arc::new(vec![])));

    // @map
    assert_eq!(q("@map key k @first"), Ok(Map(MappingTarget::Key(s!("k")), vec![o!("@first")])));
    assert_eq!(p("@map k k @next"), Map(MappingTarget::Key(s!("k")), vec![o!("@next")]));
    assert_eq!(p("@map key k @next"), Map(MappingTarget::Key(s!("k")), vec![o!("@next")]));
    assert_eq!(q("@map mouse 6 @last"), Ok(Map(MappingTarget::Mouse(6, None), vec![o!("@last")])));
    assert_eq!(p("@map m 6 @last"), Map(MappingTarget::Mouse(6, None), vec![o!("@last")]));
    assert_eq!(p("@map m --area 0.1x0.2-0.3x0.4 6 @last"), Map(MappingTarget::Mouse(6, Some(Area::new(0.1, 0.2, 0.3, 0.4))), vec![o!("@last")]));

    // Expand
    assert_eq!(p("@expand /foo/bar.txt"), Expand(false, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(p("@expand"), Expand(false, None));
    assert_eq!(p("@expand --recursive /foo/bar.txt"), Expand(true, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(p("@expand --recursive"), Expand(true, None));

    // Option
    assert_eq!(p("@toggle status"), UpdateOption(StateName::StatusBar, StateUpdater::Toggle));
    assert_eq!(p("@toggle status-bar"), UpdateOption(StateName::StatusBar, StateUpdater::Toggle));
    assert_eq!(p("@enable center"), UpdateOption(StateName::CenterAlignment, StateUpdater::Enable));
    assert_eq!(p("@disable center-alignment"), UpdateOption(StateName::CenterAlignment, StateUpdater::Disable));
    assert_eq!(p("@disable fit"), UpdateOption(StateName::Fit, StateUpdater::Disable));

    // Multi
    assert_eq!(p("; @first ; @next"), Multi(vec![First(None, false), Next(None, false)]));
    assert_eq!(p("@multi / @first / @next"), Multi(vec![First(None, false), Next(None, false)]));

    // Shell
    assert_eq!(p("@shell ls -l -a"), Shell(false, false, vec![o!("ls"), o!("-l"), o!("-a")]));
    assert_eq!(p("@shell --async ls -l -a"), Shell(true, false, vec![o!("ls"), o!("-l"), o!("-a")]));
    assert_eq!(p("@shell --async --operation ls -l -a"), Shell(true, true, vec![o!("ls"), o!("-l"), o!("-a")]));

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
