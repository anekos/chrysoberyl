
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::io::sink;

use argparse::{ArgumentParser, Store, StoreConst, StoreTrue, StoreOption, List};
use cmdline_parser::Parser;
use shellexpand;

use archive::ArchiveEntry;
use command;
use mapping::{self, InputType};
use options::AppOptionName;



#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Command(command::Command),
    Count(Option<usize>),
    CountDigit(u8),
    Expand(bool, Option<PathBuf>), /* recursive, base */
    First,
    Input(mapping::Input),
    Last,
    LazyDraw(u64), /* serial */
    Map(mapping::Input, Box<Operation>),
    Multi(Vec<Operation>),
    Next,
    Nop,
    Previous,
    PrintEntries,
    Push(String),
    PushArchiveEntry(PathBuf, ArchiveEntry),
    PushHttpCache(PathBuf, String),
    PushPath(PathBuf),
    PushURL(String),
    Quit,
    Refresh,
    Shell(bool, bool, String, Vec<String>), /* async, operation, command_name, arguments */
    Shuffle(bool), /* Fix current */
    Sort,
    Toggle(AppOptionName),
    User(Vec<(String, String)>),
    Views,
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

        Operation::from_str(s).unwrap_or(Operation::Push(expand(s)))
    }

    fn user(args: Vec<String>) -> Operation {
        let mut result: Vec<(String, String)> = vec![];
        let mut index = 0;

        for  arg in args.iter() {
            let sep = arg.find("=").unwrap_or(0);
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


fn parse_from_vec(whole: Vec<String>) -> Result<Operation, String> {
    use self::Operation::*;
    use command::Command::{Copy, Move};

    if let Some(head) = whole.get(0) {
        let name = &*head.to_lowercase();
        let args = whole[1..].to_vec();
        let whole = whole.clone();

        if Some('#') == name.chars().next() {
            return Ok(Nop)
        }

        match name {
            "@copy"                      => parse_copy_or_move(whole).map(|(path, if_exist)| Command(Copy(path, if_exist))),
            "@count"                     => parse_count(whole),
            "@entries"                   => Ok(PrintEntries),
            "@expand"                    => parse_expand(whole),
            "@first" | "@f"              => Ok(First),
            "@input"                     => parse_input(whole),
            "@last" | "@l"               => Ok(Last),
            "@map"                       => parse_map(whole),
            "@multi"                     => parse_multi(whole),
            "@move"                      => parse_copy_or_move(whole).map(|(path, if_exist)| Command(Move(path, if_exist))),
            "@next" | "@n"               => Ok(Next),
            "@prev" | "@p" | "@previous" => Ok(Previous),
            "@push"                      => parse_command1(whole, |it| Push(expand(&it))),
            "@pushpath"                  => parse_command1(whole, |it| PushPath(expand_to_pathbuf(&it))),
            "@pushurl"                   => parse_command1(whole, PushURL),
            "@quit"                      => Ok(Quit),
            "@refresh" | "@r"            => Ok(Refresh),
            "@shell"                     => parse_shell(whole),
            "@shuffle"                   => Ok(Shuffle(false)),
            "@sort"                      => Ok(Sort),
            "@toggle"                    => parse_toggle(whole),
            "@user"                      => Ok(Operation::user(args)),
            "@views"                     => Ok(Views),
            ";"                          => parse_multi_args(args, ";"),
            _ => Err(format!("Invalid commnad: {}", name))
        }
    } else {
        Ok(Nop)
    }
}

fn parse(s: &str) -> Result<Operation, String> {
    let ps: Vec<String> = Parser::new(s).map(|(_, it)| it).collect();
    parse_from_vec(ps)
}


fn parse_command1<T>(args: Vec<String>, op: T) -> Result<Operation, String>
where T: FnOnce(String) -> Operation {
    if let Some(arg) = args.get(1) {
        Ok(op(arg.to_owned()))
    } else {
        Err("Not enough argument".to_owned())
    }
}

fn parse_copy_or_move(args: Vec<String>) -> Result<(PathBuf, command::IfExist), String> {
    let mut destination = "".to_owned();
    let mut if_exist = command::IfExist::NewFileName;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut if_exist)
            .add_option(&["--fail", "-f"], StoreConst(command::IfExist::Fail), "Fail if file exists")
            .add_option(&["--overwrite", "-o"], StoreConst(command::IfExist::Overwrite), "Overwrite the file if file exists")
            .add_option(&["--new", "--new-file-name", "-n"], StoreConst(command::IfExist::NewFileName), "Generate new file name if file exists (default)");
        ap.refer(&mut destination).add_argument("destination", Store, "Destination directory").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        (expand_to_pathbuf(&destination).to_owned(), if_exist)
    })
}

fn parse_count(args: Vec<String>) -> Result<Operation, String> {
    let mut count: Option<usize> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut count).add_argument("count", StoreOption, "Put count");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Count(count)
    })
}

fn parse_expand(args: Vec<String>) -> Result<Operation, String> {
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

fn parse_input(args: Vec<String>) -> Result<Operation, String> {
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

fn parse_map(args: Vec<String>) -> Result<Operation, String> {
    let mut input_type = InputType::Key;
    let mut from = "".to_owned();
    let mut to: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut input_type)
            .add_option(&["--key", "-k"], StoreConst(InputType::Key), "For keyboard (default)")
            .add_option(&["--mouse-button", "-m"], StoreConst(InputType::MouseButton), "For mouse button");
        ap.refer(&mut from).add_argument("from", Store, "Map from (Key name or button number)").required();
        ap.refer(&mut to).add_argument("to", List, "Map to (Command)").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        input_type.input_from_text(&from).and_then(|input| {
            parse_from_vec(to).map(|op| {
                Operation::Map(input, Box::new(op))
            })
        })
    })
}

fn parse_multi(args: Vec<String>) -> Result<Operation, String> {
    let mut separator = "".to_owned();
    let mut commands: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut separator).add_argument("separator", Store, "Commands separator").required();
        ap.refer(&mut commands).add_argument("arguments", List, "Commands");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        parse_multi_args(commands, &separator)
    })
}

fn parse_multi_args(xs: Vec<String>, separator: &str) -> Result<Operation, String> {
    let mut ops: Vec<Vec<String>> = vec![];
    let mut buffer: Vec<String> = vec![];

    for x in  xs.into_iter() {
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
        match parse_from_vec(op) {
            Ok(op) => result.push(op),
            err => return err
        }
    }

    Ok(Operation::Multi(result))
}

fn parse_shell(args: Vec<String>) -> Result<Operation, String> {
    let mut async = false;
    let mut read_operations = false;
    let mut command = "".to_owned();
    let mut command_arguments: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async).add_option(&["--async", "-a"], StoreTrue, "Async");
        ap.refer(&mut read_operations).add_option(&["--operation", "-o"], StoreTrue, "Read operations form stdout");
        ap.refer(&mut command).add_argument("command", Store, "Command").required();
        ap.refer(&mut command_arguments).add_argument("arguments", List, "Command arguments");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Shell(async, read_operations, command, command_arguments)
    })
}

fn parse_toggle(args: Vec<String>) -> Result<Operation, String> {
    use options::AppOptionName::*;

    let mut name = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("option_name", Store, "Option name").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        match &*name.to_lowercase() {
            "information" | "info" => Ok(Operation::Toggle(ShowText)),
            "reverse" | "rev" => Ok(Operation::Toggle(Reverse)),
            _  => Err(format!("Unknown option: {}", name))
        }
    })
}

fn parse_args(parser: &mut ArgumentParser, args: Vec<String>) -> Result<(), String> {
    parser.stop_on_first_argument(true);
    parser.parse(args, &mut sink(), &mut sink()).map_err(|code| s!(code))
}

#[cfg(test)]#[test]
fn test_parse() {
    use self::Operation::*;

    fn p(s: &str) -> Operation {
        Operation::from_str_force(s)
    }

    // Simple
    assert_eq!(p("@First"), First);
    assert_eq!(p("@Next"), Next);
    assert_eq!(p("@Previous"), Previous);
    assert_eq!(p("@Prev"), Previous);
    assert_eq!(p("@Last"), Last);
    assert_eq!(p("@shuffle"), Shuffle(false));
    assert_eq!(p("@entries"), PrintEntries);
    assert_eq!(p("@refresh"), Refresh);
    assert_eq!(p("@sort"), Sort);

    // @push*
    assert_eq!(p("@push http://example.com/moge.jpg"), Push("http://example.com/moge.jpg".to_owned()));
    assert_eq!(p("@pushpath /hoge/moge.jpg"), PushPath(pathbuf("/hoge/moge.jpg")));
    assert_eq!(p("@pushurl http://example.com/moge.jpg"), PushURL("http://example.com/moge.jpg".to_owned()));

    // @map
    assert_eq!(p("@map k @first"), Map(mapping::Input::key("k"), Box::new(First)));
    assert_eq!(p("@map --key k @next"), Map(mapping::Input::key("k"), Box::new(Next)));
    assert_eq!(p("@map -k k @next"), Map(mapping::Input::key("k"), Box::new(Next)));
    assert_eq!(p("@map --mouse-button 6 @last"), Map(mapping::Input::MouseButton(6), Box::new(Last)));
    assert_eq!(p("@map -m 6 @last"), Map(mapping::Input::MouseButton(6), Box::new(Last)));

    // Expand
    assert_eq!(p("@expand /foo/bar.txt"), Expand(false, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(p("@expand"), Expand(false, None));
    assert_eq!(p("@expand --recursive /foo/bar.txt"), Expand(true, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(p("@expand --recursive"), Expand(true, None));

    // Toggle
    assert_eq!(p("@toggle info"), Toggle(AppOptionName::ShowText));
    assert_eq!(p("@toggle information"), Toggle(AppOptionName::ShowText));

    // Multi
    assert_eq!(p("; @first ; @next"), Multi(vec![First, Next]));
    assert_eq!(p("@multi / @first / @next"), Multi(vec![First, Next]));

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
    shellexpand::tilde(&s).into_owned()
}

fn expand_to_pathbuf(s: &str) -> PathBuf {
    Path::new(&expand(s)).to_path_buf()
}
