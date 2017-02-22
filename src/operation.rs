
use std::str::FromStr;
use std::path::PathBuf;

use options::AppOptionName;



#[derive(Clone, Debug)]
pub enum Operation {
    First,
    Next,
    Previous,
    Last,
    Refresh,
    Push(String),
    PushFile(PathBuf),
    PushURL(String),
    Key(u32),
    Count(u8),
    Toggle(AppOptionName),
    Expand,
    Exit
}



impl FromStr for Operation {
    type Err = ();
    fn from_str(src: &str) -> Result<Operation, ()> {
        let args: Vec<&str> = src.split("\t").collect();
        let args = (args.get(0), args.get(1), args.get(2));
        match args {
            (Some(&"Push"), Some(url), None)
                => Ok(Operation::Push(oo(url))),
            _ => Err(()),
        }
    }
}


fn oo(s: &&str) -> String {
    s.to_owned().to_owned()
}
