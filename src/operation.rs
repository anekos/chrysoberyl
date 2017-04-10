
use std::io::sink;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use argparse::{ArgumentParser, Collect, Store, StoreConst, StoreTrue, StoreOption, List, PushConst};
use cmdline_parser::Parser;
use css_color_parser::Color as CssColor;
use shellexpand;

use archive::ArchiveEntry;
use config::ConfigSource;
use entry::{Meta, MetaEntry};
use filer;
use gui::ColorTarget;
use mapping::{self, InputType, mouse_mapping};
use state::StateName;



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
    First(Option<usize>),
    Fragile(PathBuf),
    Input(mapping::Input),
    Last(Option<usize>),
    LazyDraw(u64), /* serial */
    LoadConfig(ConfigSource),
    Map(MappingTarget, Vec<String>),
    Multi(Vec<Operation>),
    Next(Option<usize>),
    Nop,
    OperateFile(filer::FileOperation),
    Previous(Option<usize>),
    PrintEntries,
    Push(String, Meta),
    PushArchiveEntry(PathBuf, ArchiveEntry),
    PushHttpCache(PathBuf, String, Meta),
    PushPath(PathBuf, Meta),
    PushURL(String, Meta),
    Quit,
    Random,
    Refresh,
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

        Operation::from_str(s).unwrap_or_else(|_| Operation::Push(expand(s), vec![]))
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
            "@first" | "@f"              => parse_command_usize1(whole, First),
            "@fragile"                   => parse_command1(whole, |it| Fragile(expand_to_pathbuf(&it))),
            "@input"                     => parse_input(whole),
            "@last" | "@l"               => parse_command_usize1(whole, Last),
            "@load"                      => parse_load(whole),
            "@map"                       => parse_map(whole),
            "@multi"                     => parse_multi(whole),
            "@move"                      => parse_copy_or_move(whole).map(|(path, if_exist)| OperateFile(Move(path, if_exist))),
            "@next" | "@n"               => parse_command_usize1(whole, Next),
            "@prev" | "@p" | "@previous" => parse_command_usize1(whole, Previous),
            "@push"                      => parse_push(whole, |it, meta| Push(expand(&it), meta)),
            "@pushpath"                  => parse_push(whole, |it, meta| PushPath(expand_to_pathbuf(&it), meta)),
            "@pushurl"                   => parse_push(whole, PushURL),
            "@quit"                      => Ok(Quit),
            "@random" | "@rand"          => Ok(Random),
            "@refresh" | "@r"            => Ok(Refresh),
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


fn parse_command1<T>(args: &[String], op: T) -> Result<Operation, String>
where T: FnOnce(String) -> Operation {
    if let Some(arg) = args.get(1) {
        Ok(op(arg.to_owned()))
    } else {
        Err("Not enough argument".to_owned())
    }
}

fn parse_command_usize1<T>(args: &[String], op: T) -> Result<Operation, String>
where T: FnOnce(Option<usize>) -> Operation {
    use utils::s;

    if let Some(arg) = args.get(1) {
        arg.parse().map(|it| op(Some(it))).map_err(|it| s(&it))
    } else {
        Ok(op(None))
    }
}

fn parse_cherenkov(args: &[String]) -> Result<Operation, String> {
    let mut radius = 0.1;
    let mut random_hue = 0.0;
    let mut n_spokes = 50;
    let mut x = None;
    let mut y = None;
    let mut color: CssColor = "blue".parse().unwrap();
    let mut clear = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut radius).add_option(&["--radius", "-r"], Store, "Radius");
        ap.refer(&mut random_hue).add_option(&["--random-hue", "-h", "--hue"], Store, "Random Hue");
        ap.refer(&mut n_spokes).add_option(&["--spokes", "-s"], Store, "Number of spokes");
        ap.refer(&mut x).add_option(&["-x"], StoreOption, "X");
        ap.refer(&mut y).add_option(&["-y"], StoreOption, "Y");
        ap.refer(&mut color).add_option(&["-c", "--color"], Store, "CSS Color");
        ap.refer(&mut clear).add_option(&["--clear"], StoreTrue, "Clear");
        parse_args(&mut ap, args)
    } .map(|_| {
        if clear {
            Operation::CherenkovClear
        } else {
            Operation::Cherenkov(
                CherenkovParameter {
                    radius: radius,
                    random_hue: random_hue,
                    n_spokes: n_spokes,
                    x: x,
                    y: y,
                    color: color
                })
        }
    })
}

fn parse_copy_or_move(args: &[String]) -> Result<(PathBuf, filer::IfExist), String> {
    let mut destination = "".to_owned();
    let mut if_exist = filer::IfExist::NewFileName;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut if_exist)
            .add_option(&["--fail", "-f"], StoreConst(filer::IfExist::Fail), "Fail if file exists")
            .add_option(&["--overwrite", "-o"], StoreConst(filer::IfExist::Overwrite), "Overwrite the file if file exists")
            .add_option(&["--new", "--new-file-name", "-n"], StoreConst(filer::IfExist::NewFileName), "Generate new file name if file exists (default)");
        ap.refer(&mut destination).add_argument("destination", Store, "Destination directory").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        (expand_to_pathbuf(&destination).to_owned(), if_exist)
    })
}

fn parse_color(args: &[String]) -> Result<Operation, String> {
    let mut target: ColorTarget = ColorTarget::WindowBackground;
    let mut color: CssColor = "white".parse().unwrap();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut target).add_argument("target", Store, "Target").required();
        ap.refer(&mut color).add_argument("color", Store, "CSS Color").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Color(target, color)
    })
}

fn parse_count(args: &[String]) -> Result<Operation, String> {
    let mut count: Option<usize> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut count).add_argument("count", StoreOption, "Put count");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Count(count)
    })
}

fn parse_editor(args: &[String]) -> Result<Operation, String> {
    let mut config_sources: Vec<ConfigSource> = vec![];
    let mut command_line: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut config_sources)
            .add_option(&["--user", "-u"], PushConst(ConfigSource::User), "Insert user config")
            .add_option(&["--default", "-d"], PushConst(ConfigSource::Default), "Insert defult config");
        ap.refer(&mut command_line).add_argument("command-line", StoreOption, "Command line to open editor");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Editor(command_line, config_sources)
    })
}

fn parse_expand(args: &[String]) -> Result<Operation, String> {
    let mut recursive = false;
    let mut base: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut recursive).add_option(&["--recursive", "-r", "--recur", "--rec"], StoreTrue, "Recursive");
        ap.refer(&mut base).add_argument("base-path", StoreOption, "Base path");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Expand(recursive, base.map(|it| pathbuf(&it)))
    })
}

fn parse_input(args: &[String]) -> Result<Operation, String> {
    let mut input_type = InputType::Key;
    let mut input = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut input_type)
            .add_option(&["--key", "-k"], StoreConst(InputType::Key), "For keyboard (default)")
            .add_option(&["--mouse-button", "-m"], StoreConst(InputType::MouseButton), "For mouse button");
        ap.refer(&mut input).add_argument("input", Store, "Input").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        input_type.input_from_text(&input).map(|input| {
            Operation::Input(input)
        })
    })
}

fn parse_load(args: &[String]) -> Result<Operation, String> {
    let mut config_source = ConfigSource::Default;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut config_source)
            .add_option(&["--user", "-u"], StoreConst(ConfigSource::User), "Load user config (rc.conf)")
            .add_option(&["--default", "-d"], StoreConst(ConfigSource::Default), "Load default config");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::LoadConfig(config_source)
    })
}

fn parse_map(args: &[String]) -> Result<Operation, String> {
    fn parse_map_key(args: &[String]) -> Result<Operation, String> {
        let mut from = "".to_owned();
        let mut to: Vec<String> = vec![];
        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target key name").required();
            ap.refer(&mut to).add_argument("to", List, "Command").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::Map(MappingTarget::Key(from), to)
        })
    }

    fn parse_map_mouse(args: &[String]) -> Result<Operation, String> {
        let mut from = 1;
        let mut to: Vec<String> = vec![];
        let mut area: Option<mouse_mapping::Area> = None;

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target button").required();
            ap.refer(&mut area).add_option(&["--area", "-a"], StoreOption, "Area");
            ap.refer(&mut to).add_argument("to", List, "Command").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::Map(MappingTarget::Mouse(from, area), to)
        })
    }

    if let Some(target) = args.get(1) {
        let args = &args[1..];
        match &**target {
            "k" | "key" => parse_map_key(args),
            "m" | "button" | "mouse" | "mouse-button" => parse_map_mouse(args),
            _ => Err(format!("Invalid mapping target: {}", target))
        }
    } else {
        Err(o!("Not enough arguments"))
    }
}

fn parse_multi(args: &[String]) -> Result<Operation, String> {
    let mut separator = "".to_owned();
    let mut commands: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut separator).add_argument("separator", Store, "Commands separator").required();
        ap.refer(&mut commands).add_argument("arguments", List, "Commands");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        parse_multi_args(&commands, &separator)
    })
}

fn parse_multi_args(xs: &[String], separator: &str) -> Result<Operation, String> {
    let mut ops: Vec<Vec<String>> = vec![];
    let mut buffer: Vec<String> = vec![];

    for x in xs {
        if x == separator {
            ops.push(buffer.clone());
            buffer.clear();
        } else {
            buffer.push(x.clone());
        }
    }

    if !buffer.is_empty() {
        ops.push(buffer);
    }

    let mut result: Vec<Operation> = vec![];

    for op in ops {
        match parse_from_vec(&op) {
            Ok(op) => result.push(op),
            err => return err
        }
    }

    Ok(Operation::Multi(result))
}

fn parse_option_updater(args: &[String], modifier: StateUpdater) -> Result<Operation, String> {
    use state::StateName::*;
    use self::Operation::UpdateOption;

    let mut name = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("option_name", Store, "Option name").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        match &*name.to_lowercase() {
            "status-bar" | "status" => Ok(UpdateOption(StatusBar, modifier)),
            "reverse" | "rev" => Ok(UpdateOption(Reverse, modifier)),
            "center" | "center-alignment" => Ok(UpdateOption(CenterAlignment, modifier)),
            _  => Err(format!("Unknown option: {}", name))
        }
    })
}

fn parse_push<T>(args: &[String], op: T) -> Result<Operation, String>
where T: FnOnce(String, Meta) -> Operation {
    impl FromStr for MetaEntry {
        type Err = String;

        fn from_str(src: &str) -> Result<MetaEntry, String> {
            Ok({
                if let Some(sep) = src.find("=") {
                    let (key, value) = src.split_at(sep);
                    MetaEntry { key: o!(key), value: o!(value) }
                } else {
                    MetaEntry::new_without_value(o!(src))
                }
            })
        }
    }

    let mut meta: Meta = vec![];
    let mut path: String = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut path).add_argument("Path", Store, "Path to resource").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        op(path, meta)
    })
}

fn parse_shell(args: &[String]) -> Result<Operation, String> {
    let mut async = false;
    let mut read_operations = false;
    let mut command_line: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async).add_option(&["--async", "-a"], StoreTrue, "Async");
        ap.refer(&mut read_operations).add_option(&["--operation", "-o"], StoreTrue, "Read operations form stdout");
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        parse_args(&mut ap, args)
    } .map(|_| {
        let command_line = command_line.iter().map(|it| expand(it)).collect();
        Operation::Shell(async, read_operations, command_line)
    })
}

fn parse_views(args: &[String]) -> Result<Operation, String> {
    let mut for_rows = false;
    let mut rows = None;
    let mut cols = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut for_rows).add_option(&["--rows", "-r"], StoreTrue, "Set rows");
        ap.refer(&mut cols).add_argument("columns", StoreOption, "Columns");
        ap.refer(&mut rows).add_argument("rows", StoreOption, "Rows");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if Some(0) == cols || Some(0) == rows {
            return Err(o!("Columns / rows must be greater than 0"))
        }
        Ok(
            if cols.is_some() || rows.is_some() {
                if for_rows {
                    Operation::Views(rows, cols)
                } else {
                    Operation::Views(cols, rows)
                }
            } else {
                Operation::ViewsFellow(for_rows)
            }
        )
    })
}

fn parse_args(parser: &mut ArgumentParser, args: &[String]) -> Result<(), String> {
    parser.stop_on_first_argument(true);
    parser.parse(args.to_vec(), &mut sink(), &mut sink()).map_err(|code| s!(code))
}

#[cfg(test)]#[test]
fn test_parse() {
    use self::Operation::*;
    use mapping::mouse_mapping::Area;

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
    assert_eq!(p("@editor"), Editor);

    // Move
    assert_eq!(p("@First"), First(None));
    assert_eq!(p("@Next"), Next(None));
    assert_eq!(p("@Previous"), Previous(None));
    assert_eq!(p("@Prev"), Previous(None));
    assert_eq!(p("@Last"), Last(None));
    assert_eq!(p("@First 1"), First(Some(1)));
    assert_eq!(p("@Next 2"), Next(Some(2)));
    assert_eq!(p("@Previous 3"), Previous(Some(3)));
    assert_eq!(p("@Prev 4"), Previous(Some(4)));
    assert_eq!(p("@Last 5"), Last(Some(5)));

    // @push*
    assert_eq!(p("@push http://example.com/moge.jpg"), Push("http://example.com/moge.jpg".to_owned()));
    assert_eq!(p("@pushpath /hoge/moge.jpg"), PushPath(pathbuf("/hoge/moge.jpg")));
    assert_eq!(p("@pushurl http://example.com/moge.jpg"), PushURL("http://example.com/moge.jpg".to_owned()));

    // @map
    assert_eq!(q("@map key k @first"), Ok(Map(MappingTarget::Key(s!("k")), Box::new(First(None)))));
    assert_eq!(p("@map k k @next"), Map(MappingTarget::Key(s!("k")), Box::new(Next(None))));
    assert_eq!(p("@map key k @next"), Map(MappingTarget::Key(s!("k")), Box::new(Next(None))));
    assert_eq!(q("@map mouse 6 @last"), Ok(Map(MappingTarget::Mouse(6, None), Box::new(Last(None)))));
    assert_eq!(p("@map m 6 @last"), Map(MappingTarget::Mouse(6, None), Box::new(Last(None))));
    assert_eq!(p("@map m --area 0.1x0.2-0.3x0.4 6 @last"), Map(MappingTarget::Mouse(6, Some(Area::new(0.1, 0.2, 0.3, 0.4))), Box::new(Last(None))));

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

    // Multi
    assert_eq!(p("; @first ; @next"), Multi(vec![First(None), Next(None)]));
    assert_eq!(p("@multi / @first / @next"), Multi(vec![First(None), Next(None)]));

    // Shell
    assert_eq!(p("@shell ls -l -a"), Shell(false, false, s!("ls"), vec![s!("-l"), s!("-a")]));
    assert_eq!(p("@shell --async ls -l -a"), Shell(true, false, s!("ls"), vec![s!("-l"), s!("-a")]));
    assert_eq!(p("@shell --async --operation ls -l -a"), Shell(true, true, s!("ls"), vec![s!("-l"), s!("-a")]));

    // Invalid command
    assert_eq!(p("Meow Meow"), Push("Meow Meow".to_owned()));
    assert_eq!(p("expand /foo/bar.txt"), Push("expand /foo/bar.txt".to_owned()));

    // Shell quotes
    assert_eq!(
        p(r#"@Push "http://example.com/sample.png""#),
        Push("http://example.com/sample.png".to_owned()));

    // Shell quotes
    assert_eq!(
        p(r#"@Push 'http://example.com/sample.png'"#),
        Push("http://example.com/sample.png".to_owned()));

    // Ignore leftover arguments
    assert_eq!(
        p(r#"@Push "http://example.com/sample.png" CAT IS PRETTY"#),
        Push("http://example.com/sample.png".to_owned()));

    // Ignore case
    assert_eq!(p("@ShuFFle"), Shuffle(false));
}

fn pathbuf(s: &str) -> PathBuf {
    Path::new(s).to_path_buf()
}

fn expand(s: &str) -> String {
    shellexpand::full(&s).unwrap().into_owned()
}

fn expand_to_pathbuf(s: &str) -> PathBuf {
    Path::new(&expand(s)).to_path_buf()
}
