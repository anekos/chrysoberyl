
use std::default::Default;
use std::env::{args, Args};
use std::process::exit;

use encoding::label::encoding_from_whatwg_label;
use encoding::types::EncodingRef;

use crate::app_path;
use crate::constant::{README, APPLICATION_NAME};
use crate::controller;
use crate::expandable::Expandable;



#[derive(Clone)]
pub struct Initial {
    pub config_file: Option<String>,
    pub curl_threads: u8,
    pub encodings: Vec<EncodingRef>,
    pub enforce_gtk_theme: bool,
    pub entries: Vec<Entry>,
    pub load_config: bool,
    pub shuffle: bool,
    pub silent: bool,
    pub stdin_as_binary: bool,
    pub window_role: String,
}

#[derive(Clone)]
pub enum Entry {
    Operation(Vec<String>),
    Path(String),
    Expand(String, bool),
    Controller(controller::Source),
}


impl Default for Initial {
    fn default() -> Self {
        Initial {
            config_file: None,
            curl_threads: 3,
            encodings: vec![],
            enforce_gtk_theme: true,
            entries: vec![],
            load_config: true,
            shuffle: false,
            silent: false,
            stdin_as_binary: false,
            window_role: APPLICATION_NAME.to_string(),
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

        if arg.starts_with("@@") || arg == "@@" {
            if let Some(ref op) = op {
                result.entries.push(Entry::Operation(op.clone()));
            }
            if 2 < arg.len() {
                op = Some(vec![format!("@{}", &arg[2..])]);
            } else {
                op = None;
            }
            continue;
        }

        if let Some(op) = op.as_mut() {
            op.push(arg);
        } else if arg == "-" {
            result.stdin_as_binary = true;
        } else {
            result.entries.push(Entry::Path(arg));
        }
    }

    if let Some(ref op) = op {
        result.entries.push(Entry::Operation(op.clone()));
    }

    Ok(result)
}

fn parse_option(arg: &str, args: &mut Args, init: &mut Initial) -> Result<bool, String> {
    let not_enough = || {
        Err(format!("Not enough argument for: {}", arg))
    };

    match arg {
        "--version" | "-v" => {
            print_version();
            exit(0);
        },
        "--print-path" => {
            print_path();
            exit(0);
        },
        "--help" | "-h" => {
            print_help();
            exit(0);
        },
        "--config" => if let Some(value) = args.next() {
            init.config_file = Some(value);
        } else {
            return not_enough();
        },
        "--no-config" => init.load_config = false,
        "--role" => if let Some(value) = args.next() {
            init.window_role = value;
        } else {
            return not_enough();
        },
        "--shuffle" | "-z" => init.shuffle = true,
        "--silent" | "-s" => init.silent = true,
        "--expand" | "-e" => if let Some(value) = args.next() {
            init.entries.push(Entry::Expand(value, false));
        },
        "--expand-recursive" | "-E" => if let Some(value) = args.next() {
            init.entries.push(Entry::Expand(value, true));
        },
        "--input" | "-i" => if let Some(value) = args.next() {
            init.entries.push(Entry::Controller(controller::Source::File(Expandable::expanded(value))));
        } else {
            return not_enough();
        },
        "--max-curl-threads" | "-t" => if let Some(value) = args.next() {
            match value.parse() {
                Ok(value) => init.curl_threads = value,
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
        "--use-gtk-theme" => init.enforce_gtk_theme = false,
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
        app_path::config_file().to_str().unwrap(),
        app_path::cache_dir("/").to_str().unwrap());
}

fn print_help() {
    use std::io::{self, Write};

    let mut stdout = io::stdout();
    let mut phase = 0;

    let _ = writeln!(&mut stdout, "Usage:");

    for line in README.lines() {
        match phase {
            0 if line == "# Command line" => phase = 1,
            1 if line == "```" => phase = 2,
            2 if line == "```" => phase = 3,
            2 => { let _ = writeln!(&mut stdout, "  {}", line); },
            3 => { let _ = writeln!(&mut stdout, "{}", line); },
            _ => (),
        }
    }
}
