
use std::default::Default;
use std::env::{args, Args};

use encoding::label::encoding_from_whatwg_label;
use encoding::types::EncodingRef;

use app_path;



pub struct Initial {
    pub http_threads: u8,
    pub shuffle: bool,
    pub encodings: Vec<EncodingRef>,
    pub entries: Vec<Entry>,
    pub silent: bool,
}


pub enum Entry {
    Operation(Vec<String>),
    Path(String),
    Expand(String, bool),
    Input(String),
}

impl Default for Initial {
    fn default() -> Self {
        Initial {
            http_threads: 3,
            shuffle: false,
            encodings: vec![],
            entries: vec![],
            silent: false,
        }
    }
}



pub fn parse_args() -> Result<Initial, String> {
    let mut op: Option<Vec<String>> = None;
    let mut result = Initial::default();
    let mut args = args();

    let _ = args.next();

    while let Some(arg) = args.next() {
        match parse_option(&arg, &mut args, &mut result) {
            Ok(true) => continue,
            Ok(false) => (),
            Err(err) => return Err(err),
        }

        if arg.starts_with("--") {
            if let Some(ref op) = op {
                result.entries.push(Entry::Operation(op.clone()));
            }
            if 2 < arg.len() {
                op = Some(vec![o!(arg[2..])])
            } else {
                op = None;
            }
            continue;
        }

        if let Some(op) = op.as_mut() {
            op.push(arg);
        } else {
            result.entries.push(Entry::Path(arg));
        }
    }

    Ok(result)
}

fn parse_option(arg: &str, args: &mut Args, init: &mut Initial) -> Result<bool, String> {
    let not_enough = || {
        Err(format!("Not enough argument for: {}", arg))
    };

    match arg {
        "--version" | "-v" => print_version(),
        "--print-path" => print_path(),
        "--shuffle" | "-z" => init.shuffle = true,
        "--silent" => init.silent = true,
        "--expand" | "-e" => if let Some(value) = args.next() {
            init.entries.push(Entry::Expand(value, false));
        },
        "--expand-recursive" | "-E" => if let Some(value) = args.next() {
            init.entries.push(Entry::Expand(value, true));
        },
        "--input" | "-i" => if let Some(value) = args.next() {
            init.entries.push(Entry::Input(value));
        } else {
            return not_enough();
        },
        "--max-http-threads" | "-t" => if let Some(value) = args.next() {
            match value.parse() {
                Ok(value) => init.http_threads = value,
                Err(err) => return Err(s!(err)),
            }
        } else {
            return not_enough();
        },
        "--encoding" => if let Some(value) = args.next() {
            if let Some(encoding) = encoding_from_whatwg_label(&value) {
                init.encodings.push(encoding);
            } else {
                return Err(format!("invalid_encoding_name: {}", value));
            }
        } else {
            return not_enough();
        },
        _ => return Ok(false)
    }

    Ok(true)
}

fn print_version() {
    println!("{}", env!("CARGO_PKG_VERSION").to_string());
}

fn print_path() {
    println!(
        "configuration: {}\ncache: {}",
        app_path::config_file(None).to_str().unwrap(),
        app_path::cache_dir("/").to_str().unwrap());
}
