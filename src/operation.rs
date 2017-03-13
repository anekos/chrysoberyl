
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use cmdline_parser::Parser;

use archive::ArchiveEntry;
use key::KeyData;
use mapping::Input;
use options::AppOptionName;



#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    First,
    Next,
    Previous,
    Last,
    Refresh,
    Push(String),
    PushPath(PathBuf),
    PushHttpCache(PathBuf, String),
    PushURL(String),
    PushArchiveEntry(PathBuf, ArchiveEntry, Arc<Vec<u8>>),
    Key(KeyData),
    Button(u32),
    CountDigit(u8),
    Toggle(AppOptionName),
    Expand(Option<PathBuf>),
    ExpandRecursive(Option<PathBuf>),
    Shuffle(bool), /* Fix current */
    User(Vec<(String, String)>),
    Map(Input, Box<Operation>),
    PrintEntries,
    Sort,
    Quit,
    Nop
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


fn parse_from_vec(whole: Vec<String>) -> Option<Operation> {
    use self::Operation::*;

    fn pb(args: Vec<String>, index: usize) -> Option<PathBuf> {
        args.get(index).map(|it| pathbuf(it))
    }

    if let Some(head) = whole.get(0) {
        let name = &*head.to_lowercase();
        let args = whole[1..].to_vec();

        if Some('#') == name.chars().next() {
            return Some(Nop)
        }

        match name {
            "@push" => iter_let!(args => [path] {
                Some(Push(path.to_owned()))
            }),
            "@pushpath" => iter_let!(args => [path] {
                Some(PushPath(pathbuf(path)))
            }),
            "@pushurl" => iter_let!(args => [path] {
                Some(PushURL(path.to_owned()))
            }),
            "@map" => iter_let!(args => [kind, name] {
                match &*kind.to_lowercase() {
                    "key" | "keyboard"                  => Some(Input::key(name)),
                    "button" | "mouse" | "mouse_button" => name.parse().ok().map(|button| Input::mouse_button(button)),
                    _                                   => None
                } .and_then(|input| {
                    parse_from_vec(args.map(|it| it.to_owned()).collect()).map(|op| {
                        Map(input, Box::new(op))
                    })
                })
            }),
            "@toggle" => iter_let!(args => [name] {
                use options::AppOptionName::*;
                match &*name.to_lowercase() {
                    "info" | "information" => Some(Toggle(ShowText)),
                    _                      => None
                }
            }),
            "@next" | "@n"               => Some(Next),
            "@prev" | "@p" | "@previous" => Some(Previous),
            "@first" | "@f"              => Some(First),
            "@last" | "@l"               => Some(Last),
            "@refresh" | "@r"            => Some(Refresh),
            "@shuffle"                   => Some(Shuffle(false)),
            "@entries"                   => Some(PrintEntries),
            "@sort"                      => Some(Sort),
            "@expand"                    => Some(Expand(pb(args, 0))),
            "@expandrecursive"           => Some(ExpandRecursive(pb(args, 0))),
            "@quit"                      => Some(Quit),
            "@user"                      => Some(Operation::user(args)),
            _ => None
        }
    } else {
        Some(Nop)
    }
}


fn parse(s: &str) -> Operation {
    use self::Operation::*;

    let ps: Vec<String> = Parser::new(s).map(|(_, it)| it).collect();
    parse_from_vec(ps).unwrap_or(Push(s.to_owned()))
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

    // 1 argument
    assert_eq!(parse("@push http://example.com/moge.jpg"), Push("http://example.com/moge.jpg".to_owned()));
    assert_eq!(parse("@pushpath /hoge/moge.jpg"), PushPath(pathbuf("/hoge/moge.jpg")));
    assert_eq!(parse("@pushurl http://example.com/moge.jpg"), PushURL("http://example.com/moge.jpg".to_owned()));

    // 1 optional argument
    assert_eq!(parse("@expand /foo/bar.txt"), Expand(Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(parse("@expand"), Expand(None));
    assert_eq!(parse("@expandrecursive /foo/bar.txt"), ExpandRecursive(Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(parse("@expandrecursive"), ExpandRecursive(None));

    // Invalid commands be Push
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
