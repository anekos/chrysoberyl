
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::io::sink;

use argparse::{ArgumentParser, Store, StoreConst, StoreTrue, StoreOption, List};
use cmdline_parser::Parser;

use archive::ArchiveEntry;
use key::KeyData;
use mapping::Input;
use options::AppOptionName;



#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Button(u32),
    Count(Option<usize>),
    CountDigit(u8),
    Expand(bool, Option<PathBuf>), /* recursive, base */
    First,
    Key(KeyData),
    Last,
    Map(Input, Box<Operation>),
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
    Script(bool, String, Vec<String>), /* async, command_name, arguments */
    Shuffle(bool), /* Fix current */
    Sort,
    Toggle(AppOptionName),
    User(Vec<(String, String)>),
}



impl FromStr for Operation {
    type Err = ();
    fn from_str(src: &str) -> Result<Operation, ()> {
        Ok(parse(src))
    }
}


impl Operation {
    pub fn from_str_force(s: &str) -> Operation {
        use std::str::FromStr;

        Operation::from_str(s).unwrap_or(Operation::Push(s.to_owned()))
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

    if let Some(head) = whole.get(0) {
        let name = &*head.to_lowercase();
        let args = whole[1..].to_vec();
        let whole = whole.clone();

        if Some('#') == name.chars().next() {
            return Ok(Nop)
        }

        match name {
            "@count"                     => parse_count(whole),
            "@entries"                   => Ok(PrintEntries),
            "@expand"                    => parse_expand(whole),
            "@first" | "@f"              => Ok(First),
            "@last" | "@l"               => Ok(Last),
            "@map"                       => parse_map(whole),
            "@multi"                     => parse_multi(whole),
            "@next" | "@n"               => Ok(Next),
            "@prev" | "@p" | "@previous" => Ok(Previous),
            "@push"                      => parse_command1(whole, Push),
            "@pushpath"                  => parse_command1(whole, |it| PushPath(pathbuf(&it))),
            "@pushurl"                   => parse_command1(whole, PushURL),
            "@quit"                      => Ok(Quit),
            "@refresh" | "@r"            => Ok(Refresh),
            "@script"                    => parse_script(whole),
            "@shuffle"                   => Ok(Shuffle(false)),
            "@sort"                      => Ok(Sort),
            "@toggle"                    => parse_toggle(whole),
            "@user"                      => Ok(Operation::user(args)),
            ";"                          => parse_multi_args(args, ";"),
            _ => Err(format!("Invalid commnad: {}", name))
        }
    } else {
        Ok(Nop)
    }
}

fn parse(s: &str) -> Operation {
    use self::Operation::*;

    let ps: Vec<String> = Parser::new(s).map(|(_, it)| it).collect();
    parse_from_vec(ps).unwrap_or(Push(s.to_owned()))
}


fn parse_command1<T>(args: Vec<String>, op: T) -> Result<Operation, String>
where T: FnOnce(String) -> Operation {
    if let Some(arg) = args.get(1) {
        Ok(op(arg.to_owned()))
    } else {
        Err("Not enough argument".to_owned())
    }
}

fn parse_count(args: Vec<String>) -> Result<Operation, String> {
    let mut count: Option<usize> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut count).add_argument("count", StoreOption, "Put count");
        parse_args(ap, args)
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
        parse_args(ap, args)
    } .map(|_| {
        Operation::Expand(recursive, base.map(|it| pathbuf(&it)))
    })
}

fn parse_map(args: Vec<String>) -> Result<Operation, String> {
    #[derive(Clone, Copy)]
    enum InputType {
        Keyboard,
        MouseButton
    }

    let mut input_type = InputType::Keyboard;
    let mut from = "".to_owned();
    let mut to: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut input_type)
            .add_option(&["--keyboard", "-k"], StoreConst(InputType::Keyboard), "For keyboard (default)")
            .add_option(&["--mouse-button", "-m"], StoreConst(InputType::MouseButton), "For mouse button");
        ap.refer(&mut from).add_argument("from", Store, "Map from (Key name or button number)").required();
        ap.refer(&mut to).add_argument("to", List, "Map to (Command)").required();
        parse_args(ap, args)
    } .and_then(|_| {
        match input_type {
            InputType::Keyboard => Ok(Input::key(&from)),
            InputType::MouseButton => {
                match from.parse() {
                    Ok(button) => Ok(Input::mouse_button(button)),
                    Err(err) => Err(s!(err)),
                }
            }
        } .and_then(|input| {
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
        parse_args(ap, args)
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

fn parse_script(args: Vec<String>) -> Result<Operation, String> {
    let mut async = false;
    let mut command = "".to_owned();
    let mut command_arguments: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async).add_option(&["--async", "-a"], StoreTrue, "Async");
        ap.refer(&mut command).add_argument("command", Store, "Command").required();
        ap.refer(&mut command_arguments).add_argument("arguments", List, "Command arguments");
        parse_args(ap, args)
    } .map(|_| {
        Operation::Script(async, command, command_arguments)
    })
}

fn parse_toggle(args: Vec<String>) -> Result<Operation, String> {
    use options::AppOptionName::*;

    let mut name = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("option_name", Store, "Option name").required();
        parse_args(ap, args)
    } .and_then(|_| {
        match &*name.to_lowercase() {
            "info" | "information" => Ok(Operation::Toggle(ShowText)),
            _  => Err(format!("Unknown option: {}", name))
        }
    })
}

fn parse_args(parser: ArgumentParser, args: Vec<String>) -> Result<(), String> {
    parser.parse(args, &mut sink(), &mut sink()).map_err(|code| s!(code))
}

#[cfg(test)]#[test]
fn test_parse() {
    use self::Operation::*;

    // Simple
    assert_eq!(parse("@First"), First);
    assert_eq!(parse("@Next"), Next);
    assert_eq!(parse("@Previous"), Previous);
    assert_eq!(parse("@Prev"), Previous);
    assert_eq!(parse("@Last"), Last);
    assert_eq!(parse("@shuffle"), Shuffle(false));
    assert_eq!(parse("@entries"), PrintEntries);
    assert_eq!(parse("@refresh"), Refresh);
    assert_eq!(parse("@sort"), Sort);

    // @push*
    assert_eq!(parse("@push http://example.com/moge.jpg"), Push("http://example.com/moge.jpg".to_owned()));
    assert_eq!(parse("@pushpath /hoge/moge.jpg"), PushPath(pathbuf("/hoge/moge.jpg")));
    assert_eq!(parse("@pushurl http://example.com/moge.jpg"), PushURL("http://example.com/moge.jpg".to_owned()));

    // @map
    assert_eq!(parse("@map k @first"), Map(Input::key("k"), Box::new(First)));
    assert_eq!(parse("@map --keyboard k @next"), Map(Input::key("k"), Box::new(Next)));
    assert_eq!(parse("@map -k k @next"), Map(Input::key("k"), Box::new(Next)));
    assert_eq!(parse("@map --mouse-button 6 @last"), Map(Input::MouseButton(6), Box::new(Last)));
    assert_eq!(parse("@map -m 6 @last"), Map(Input::MouseButton(6), Box::new(Last)));

    // Expand
    assert_eq!(parse("@expand /foo/bar.txt"), Expand(false, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(parse("@expand"), Expand(false, None));
    assert_eq!(parse("@expand --recursive /foo/bar.txt"), Expand(true, Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(parse("@expand --recursive"), Expand(true, None));

    // Toggle
    assert_eq!(parse("@toggle info"), Toggle(AppOptionName::ShowText));
    assert_eq!(parse("@toggle information"), Toggle(AppOptionName::ShowText));

    // Multi
    assert_eq!(parse("; @first ; @next"), Multi(vec![First, Next]));
    assert_eq!(parse("@multi / @first / @next"), Multi(vec![First, Next]));

    // Invalid command
    assert_eq!(parse("Meow Meow"), Push("Meow Meow".to_owned()));
    assert_eq!(parse("expand /foo/bar.txt"), Push("expand /foo/bar.txt".to_owned()));

    // Shell quotes
    assert_eq!(
        parse(r#"@Push "http://example.com/sample.png""#),
        Push("http://example.com/sample.png".to_owned()));

    // Shell quotes
    assert_eq!(
        parse(r#"@Push 'http://example.com/sample.png'"#),
        Push("http://example.com/sample.png".to_owned()));

    // Ignore leftover arguments
    assert_eq!(
        parse(r#"@Push "http://example.com/sample.png" CAT IS PRETTY"#),
        Push("http://example.com/sample.png".to_owned()));

    // Ignore case
    assert_eq!(parse("@ShuFFle"), Shuffle(false));
}

fn pathbuf(s: &str) -> PathBuf {
    Path::new(s).to_path_buf()
}
