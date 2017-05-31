
use std::io::sink;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use argparse::{ArgumentParser, Collect, Store, StoreConst, StoreTrue, StoreFalse, StoreOption, List};

use color::Color;
use entry::{Meta, MetaEntry, SearchKey, new_opt_meta};
use filer;
use mapping::{Input, InputType, mouse_mapping};
use script::{ConfigSource, ScriptSource};
use shellexpand_wrapper as sh;

use operation::*;



pub fn parse_command1<T>(args: &[String], op: T) -> Result<Operation, String>
where T: FnOnce(String) -> Operation {
    if let Some(arg) = args.get(1) {
        Ok(op(arg.to_owned()))
    } else {
        Err("Not enough argument".to_owned())
    }
}

pub fn parse_move<T>(args: &[String], op: T) -> Result<Operation, String>
where T: FnOnce(Option<usize>, bool, MoveBy, bool) -> Operation {
    let mut ignore_views = false;
    let mut count = None;
    let mut move_by = MoveBy::Page;
    let mut wrap = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut ignore_views).add_option(&["--ignore-views", "-i"], StoreTrue, "Ignore the number of views");
        ap.refer(&mut wrap).add_option(&["--wrap", "-w"], StoreTrue, "First/Last page to Last/First page");
        ap.refer(&mut move_by).add_option(&["--archive", "-a"], StoreConst(MoveBy::Archive), "Set move unit to `archive`");
        ap.refer(&mut count).add_argument("count", StoreOption, "Count");
        parse_args(&mut ap, args)
    } .map(|_| {
        op(count, ignore_views, move_by, wrap)
    })
}

pub fn parse_cherenkov(args: &[String]) -> Result<Operation, String> {
    let mut radius = 0.1;
    let mut random_hue = 0.0;
    let mut n_spokes = 50;
    let mut x = None;
    let mut y = None;
    let mut color: Color = "random".parse().unwrap();
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

pub fn parse_copy_or_move(args: &[String]) -> Result<(PathBuf, filer::IfExist), String> {
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
        (o!(sh::expand_to_pathbuf(&destination)), if_exist)
    })
}

pub fn parse_clip(args: &[String]) -> Result<Operation, String> {
    let mut region = Region { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 };

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut region.left).add_argument("Left", Store, "Left");
        ap.refer(&mut region.top).add_argument("Top", Store, "Top");
        ap.refer(&mut region.right).add_argument("Right", Store, "Right");
        ap.refer(&mut region.bottom).add_argument("Bottom", Store, "Bottom");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Clip(region)
    })
}

pub fn parse_count(args: &[String]) -> Result<Operation, String> {
    let mut count: Option<usize> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut count).add_argument("count", StoreOption, "Put count");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Count(count)
    })
}

pub fn parse_editor(args: &[String]) -> Result<Operation, String> {
    let mut config_sources: Vec<ConfigSource> = vec![];
    let mut files: Vec<String> = vec![];
    let mut command_line: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut config_sources).add_option(&["--config", "-c"], Collect, "Insert config");
        ap.refer(&mut files).add_option(&["--file", "-f"], Collect, "Insert the given file");
        ap.refer(&mut command_line).add_argument("command-line", StoreOption, "Command line to open editor");
        parse_args(&mut ap, args)
    } .map(|_| {
        let mut script_sources = vec![];
        for source in config_sources {
            script_sources.push(ScriptSource::Config(source))
        }
        for file in files {
            script_sources.push(ScriptSource::File(sh::expand_to_pathbuf(&file)))
        }
        Operation::Editor(command_line, script_sources)
    })
}

pub fn parse_expand(args: &[String]) -> Result<Operation, String> {
    let mut recursive = false;
    let mut base: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut recursive).add_option(&["--recursive", "-r", "--recur", "--rec"], StoreTrue, "Recursive");
        ap.refer(&mut base).add_argument("base-path", StoreOption, "Base path");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Expand(recursive, base.map(|it| Path::new(&it).to_path_buf()))
    })
}

pub fn parse_filter(args: &[String]) -> Result<Operation, String> {
    let mut command_line: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut command_line).add_argument("command_line", List, "Command line");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        Ok(Operation::Filter(command_line))
    })
}

pub fn parse_input(args: &[String]) -> Result<Operation, String> {
    impl InputType {
        pub fn input_from_text(&self, text: &str) -> Result<Input, String> {
            match *self {
                InputType::Key =>
                    Ok(Input::key(text)),
                InputType::MouseButton => {
                    match text.parse() {
                        Ok(button) => Ok(Input::mouse_button(0, 0, button)),
                        Err(err) => Err(s!(err)),
                    }
                }
            }
        }
    }

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

pub fn parse_kill_timer(args: &[String]) -> Result<Operation, String> {
    let mut name = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("name", Store, "Name").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::KillTimer(name)
    })
}

pub fn parse_load(args: &[String]) -> Result<Operation, String> {
    let mut config_source = ConfigSource::Default;
    let mut path: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut config_source).add_option(&["--config", "-c"], Store, "Load config");
        ap.refer(&mut path).add_argument("file-path", StoreOption, "File path");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Load({
            if let Some(ref path) = path {
                ScriptSource::File(sh::expand_to_pathbuf(path))
            } else {
                ScriptSource::Config(config_source)
            }
        })
    })
}

pub fn parse_map(args: &[String]) -> Result<Operation, String> {
    fn parse_map_key(args: &[String]) -> Result<Operation, String> {
        let mut from = "".to_owned();
        let mut to: Vec<String> = vec![];
        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target key sequence").required();
            ap.refer(&mut to).add_argument("to", List, "Command").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::Map(MappingTarget::Key(from.split(',').map(|it| o!(it)).collect()), to)
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

pub fn parse_multi(args: &[String]) -> Result<Operation, String> {
    let mut separator = "".to_owned();
    let mut commands: Vec<String> = vec![];
    let mut async = true;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async)
            .add_option(&["--async", "-a"], StoreTrue, "Async")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync");
        ap.refer(&mut separator).add_argument("separator", Store, "Commands separator").required();
        ap.refer(&mut commands).add_argument("arguments", List, "Commands");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        parse_multi_args(&commands, &separator, async)
    })
}

pub fn parse_multi_args(xs: &[String], separator: &str, async: bool) -> Result<Operation, String> {
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

    let mut result: VecDeque<Operation> = VecDeque::new();

    for op in ops {
        match Operation::parse_from_vec(&op) {
            Ok(op) => result.push_back(op),
            err => return err
        }
    }

    Ok(Operation::Multi(result, async))
}

pub fn parse_option_cycle(args: &[String]) -> Result<Operation, String> {
    let mut option_name = OptionName::default();
    let mut reverse = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut reverse).add_option(&["--reverse", "-r"], StoreTrue, "Reversed cycle");
        ap.refer(&mut option_name).add_argument("option_name", Store, "Option name").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::UpdateOption(option_name, OptionUpdater::Cycle(reverse))
    })
}

pub fn parse_option_set(args: &[String]) -> Result<Operation, String> {
    let mut option_name = OptionName::default();
    let mut option_value = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut option_name).add_argument("option_name", Store, "Option name").required();
        ap.refer(&mut option_value).add_argument("option_value", Store, "Option value").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::UpdateOption(option_name, OptionUpdater::Set(option_value))
    })
}

pub fn parse_option_1(args: &[String], updater: OptionUpdater) -> Result<Operation, String> {
    let mut option_name = OptionName::default();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut option_name).add_argument("option_name", Store, "Option name").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::UpdateOption(option_name, updater)
    })
}

pub fn parse_push<T>(args: &[String], op: T) -> Result<Operation, String>
where T: FnOnce(String, Option<Meta>) -> Operation {
    impl FromStr for MetaEntry {
        type Err = String;

        fn from_str(src: &str) -> Result<MetaEntry, String> {
            Ok({
                if let Some(sep) = src.find('=') {
                    let (key, value) = src.split_at(sep);
                    MetaEntry { key: o!(key), value: o!(value[1..]) }
                } else {
                    MetaEntry::new_without_value(o!(src))
                }
            })
        }
    }

    let mut meta: Vec<MetaEntry> = vec![];
    let mut path: String = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut path).add_argument("Path", Store, "Path to resource").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        op(sh::expand(&path), new_opt_meta(meta))
    })
}

pub fn parse_save(args: &[String]) -> Result<Operation, String> {
    let mut path: Option<String> = None;
    let mut sources: Vec<StdinSource> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut sources).add_option(&["--target", "-t"], Collect, "Target");
        ap.refer(&mut path).add_argument("path", StoreOption, "Save to");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if sources.is_empty() {
            sources.push(StdinSource::Session);
        }
        Ok(Operation::Save(path.map(|it| sh::expand_to_pathbuf(&it)), sources))
    })
}

pub fn parse_set_env(args: &[String]) -> Result<Operation, String> {
    let mut name = o!("");
    let mut value: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("env-name", Store, "Env name").required();
        ap.refer(&mut value).add_argument("env-value", StoreOption, "Value");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::SetEnv(name, value.map(|it| sh::expand(&it)))
    })
}

pub fn parse_scroll(args: &[String]) -> Result<Operation, String> {
    let mut direction = Direction::Up;
    let mut operation = vec![];
    let mut scroll_size = 1.0;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut direction).add_argument("direction", Store, "left|up|right|down").required();
        ap.refer(&mut scroll_size).add_option(&["-s", "--size"], Store, "Scroll size (default 1.0) ");
        ap.refer(&mut operation).add_argument("operation", List, "Operation");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Scroll(direction, operation, scroll_size)
    })
}

pub fn parse_shell(args: &[String]) -> Result<Operation, String> {
    let mut async = true;
    let mut read_operations = false;
    let mut command_line: Vec<String> = vec![];
    let mut stdin_sources: Vec<StdinSource> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async)
            .add_option(&["--async", "-a"], StoreTrue, "Async (Non-blocking)")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync (Blocking)");
        ap.refer(&mut stdin_sources).add_option(&["--stdin", "-i"], Collect, "STDIN source");
        ap.refer(&mut read_operations).add_option(&["--operation", "-o"], StoreTrue, "Read operations form stdout");
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let mut cl: Vec<String> = vec![];
        for it in command_line {
            cl.push(sh::expand(&it));
        }
        Ok(Operation::Shell(async, read_operations, cl, stdin_sources))
    })
}

pub fn parse_show(args: &[String]) -> Result<Operation, String> {
    let mut key = SearchKey { path: o!(""), index: None };

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut key.path).add_argument("path", Store, "File path / URL");
        ap.refer(&mut key.index).add_argument("page", StoreOption, "Page");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if let Some(mut index) = key.index.as_mut() {
            if *index == 0 {
                return Err(o!("Page is 1 origin"))
            }
            *index -= 1;
        }
        Ok(Operation::Show(key))
    })
}

pub fn parse_timer(args: &[String]) -> Result<Operation, String> {
    let mut interval_seconds = 1;
    let mut name = o!("");
    let mut op = Vec::<String>::new();
    let mut repeat = Some(1);

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut repeat)
            .add_option(&["--repeat", "-r"], StoreOption, "Repeat (0 means infinitely)")
            .add_option(&["--infinity", "-i"], StoreConst(None), "Repeat infinitely");
        ap.refer(&mut name).add_argument("name", Store, "Name").required();
        ap.refer(&mut interval_seconds).add_argument("interval", Store, "Interval");
        ap.refer(&mut op).add_argument("operation", Collect, "Operation").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Timer(name, op, Duration::from_secs(interval_seconds), repeat)
    })
}

pub fn parse_views(args: &[String]) -> Result<Operation, String> {
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

pub fn parse_write(args: &[String]) -> Result<Operation, String> {
    let mut index = None;
    let mut path = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut index).add_option(&["--index", "-i"], StoreOption, "Index (1 origin)");
        ap.refer(&mut path).add_argument("path", Store, "Save to").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Write(sh::expand_to_pathbuf(&path), index)
    })
}

pub fn parse_args(parser: &mut ArgumentParser, args: &[String]) -> Result<(), String> {
    parser.stop_on_first_argument(true);
    parser.parse(args.to_vec(), &mut sink(), &mut sink()).map_err(|code| s!(code))
}


impl FromStr for StdinSource {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        match src {
            "options" | "option" | "o" =>
                Ok(StdinSource::Options),
            "entries" | "entry" | "e" =>
                Ok(StdinSource::Entries),
            "paths" | "path" | "P" =>
                Ok(StdinSource::Paths),
            "position" | "pos" | "p" =>
                Ok(StdinSource::Position),
            "mappings" | "map" | "m" =>
                Ok(StdinSource::Mappings),
            "session" | "a" =>
                Ok(StdinSource::Session),
            _ =>
                Err(format!("Invalid stdin source: {}", src))
        }
    }
}

impl FromStr for ConfigSource {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::ConfigSource::*;

        match src {
            "user" | "u" =>
                Ok(User),
            "default" | "d" =>
                Ok(Default),
            "session" | "s" =>
                Ok(DefaultSession),
            _ =>
                Err(format!("Invalid config source: {}", src))
        }
    }
}
