
use std::str::FromStr;
use std::path::{Path, PathBuf};
use cmdline_parser::Parser;

use options::AppOptionName;
use key::KeyData;



#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    First,
    Next,
    Previous,
    Last,
    Refresh,
    Push(String),
    PushFile(PathBuf),
    PushURL(String),
    Key(KeyData),
    Button(u32),
    Count(u8),
    Toggle(AppOptionName),
    Expand(Option<PathBuf>),
    ExpandRecursive(Option<PathBuf>),
    Shuffle(bool), /* Fix current */
    Sort,
    Exit
}



impl FromStr for Operation {
    type Err = ();
    fn from_str(src: &str) -> Result<Operation, ()> {
        Ok(parse(src))
    }
}


fn parse(s: &str) -> Operation {
    use self::Operation::*;

    let mut whole =  Parser::new(s).map(|(_, it)| it);

    if let Some(name) = whole.next() {
        let name = &*name.to_lowercase();
        let args: Vec<String> = whole.collect();

        match name {
            "@push" => iter_let!(args => [path] {
                return Push(path.to_owned())
            }),
            "@pushfile" => iter_let!(args => [path] {
                return PushFile(pathbuf(path))
            }),
            "@pushurl" => iter_let!(args => [path] {
                return PushURL(path.to_owned())
            }),
            "@next" | "@n"               => return Next,
            "@prev" | "@p" | "@previous" => return Previous,
            "@first" | "@f"              => return First,
            "@last" | "@l"               => return Last,
            "@refresh" | "@r"            => return Refresh,
            "@shuffle"                   => return Shuffle(true),
            "@sort"                      => return Sort,
            "@expand"                    => return Expand(args.get(0).map(|it| pathbuf(it))),
            "@expandrecursive"           => return ExpandRecursive(args.get(0).map(|it| pathbuf(it))),
            _ => ()
        }
    }

    Push(s.to_owned())
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
    assert_eq!(parse("@shuffle"), Shuffle(true));
    assert_eq!(parse("@refresh"), Refresh);
    assert_eq!(parse("@sort"), Sort);

    // 1 argument
    assert_eq!(parse("@push http://example.com/moge.jpg"), Push("http://example.com/moge.jpg".to_owned()));
    assert_eq!(parse("@pushfile /hoge/moge.jpg"), PushFile(pathbuf("/hoge/moge.jpg")));
    assert_eq!(parse("@pushurl http://example.com/moge.jpg"), PushURL("http://example.com/moge.jpg".to_owned()));

    // 1 optional argument
    assert_eq!(parse("@expand /foo/bar.txt"), Expand(Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(parse("@expand"), Expand(None));
    assert_eq!(parse("@expandrecursive /foo/bar.txt"), ExpandRecursive(Some(pathbuf("/foo/bar.txt"))));
    assert_eq!(parse("expandrecursive"), ExpandRecursive(None));

    // Invalid commands be PushFile
    assert_eq!(parse("Meow Meow"), PushFile(pathbuf("Meow Meow")));
    assert_eq!(parse("expand /foo/bar.txt"), PushFile(pathbuf("expand /foo/bar.txt")));

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
    assert_eq!(parse("@ShuFFle"), Shuffle(true));
}

fn pathbuf(s: &str) -> PathBuf {
    Path::new(s).to_path_buf()
}
