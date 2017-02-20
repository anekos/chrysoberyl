
use std::str::FromStr;

use options::AppOptionName;



#[derive(Clone, Debug)]
pub enum Operation {
    First,
    Next,
    Previous,
    Last,
    Refresh,
    Push(String),
    PushFile(String),
    PushURL(String),
    Key(u32),
    Count(u8),
    Toggle(AppOptionName),
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


impl Operation {
    pub fn log(&self, file: Option<String>) {
        use Operation::*;

        match self {
            &First => println!("First"),
            &Next => println!("Next"),
            &Previous => println!("Previous"),
            &Last => println!("Last"),
            &Refresh => println!("Refresh"),
            &Push(ref path) => println!("Push\t{}", path),
            &PushFile(ref file) => println!("PushFile\t{}", file),
            &PushURL(ref url) => println!("PushURL\t{}", url),
            &Key(key) => if let Some(file) = file {
                println!("Key\t{}\t{}", key, file);
            } else {
                println!("Key\t{}", key);
            },
            &Count(count) => println!("Count\t{}", count),
            &Toggle(ref option_name) => println!("Toggle\t{:?}", option_name),
            &Exit => println!("Exit"),
        }
    }
}


fn oo(s: &&str) -> String {
    s.to_owned().to_owned()
}
