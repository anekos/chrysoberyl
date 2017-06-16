
use std::io::sink;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use argparse::{ArgumentParser, Collect, Store, StoreConst, StoreTrue, StoreFalse, StoreOption, List};

use color::Color;
use entry::{Meta, MetaEntry, SearchKey, new_opt_meta};
use entry_filter;
use expandable::Expandable;
use filer;
use mapping::{Input, InputType};
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

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut radius).add_option(&["--radius", "-r"], Store, "Radius");
        ap.refer(&mut random_hue).add_option(&["--random-hue", "-h", "--hue"], Store, "Random Hue");
        ap.refer(&mut n_spokes).add_option(&["--spokes", "-s"], Store, "Number of spokes");
        ap.refer(&mut x).add_option(&["-x"], StoreOption, "X");
        ap.refer(&mut y).add_option(&["-y"], StoreOption, "Y");
        ap.refer(&mut color).add_option(&["-c", "--color"], Store, "CSS Color");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Cherenkov(
            CherenkovParameter {
                radius: radius,
                random_hue: random_hue,
                n_spokes: n_spokes,
                x: x,
                y: y,
                color: color
            })
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
    let d = 0.05;
    let mut region = Region { left: d, top: d, right: 1.0 - d, bottom: 1.0 - d };

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

pub fn parse_define_switch(args: &[String]) -> Result<Operation, String> {
    let mut name: String = o!("");
    let mut values_source = Vec::<String>::new();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("switch-name", Store, "Switch name").required();
        ap.refer(&mut values_source).add_argument("values", Collect, "Values").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        let mut values = vec![];
        let mut value = vec![];

        for it in values_source {
            if it == "--" {
                values.push(value);
                value = vec![];
            } else {
                value.push(o!(it));
            }
        }

        if !value.is_empty() {
            values.push(value);
        }

        Operation::DefineUserSwitch(name, values)
    })
}

pub fn parse_editor(args: &[String]) -> Result<Operation, String> {
    let mut sessions: Vec<Session> = vec![];
    let mut files: Vec<Expandable> = vec![];
    let mut command_line: Option<Expandable> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut files).add_option(&["--file", "-f"], Collect, "Insert the given file");
        ap.refer(&mut sessions).add_option(&["--session", "-S"], Collect, "Sessions");
        ap.refer(&mut command_line).add_argument("command-line", StoreOption, "Command line to open editor");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Editor(command_line, files, sessions)
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

pub fn parse_fill(args: &[String]) -> Result<Operation, String> {
    let mut cell_index = 1;
    let mut region = None;
    let mut color = Color::black();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut cell_index).add_option(&["--cell-index", "-i"], Store, "Cell index (1 origin, default = 1)");
        ap.refer(&mut region).add_option(&["--region", "-r"], StoreOption, "Fill target region");
        ap.refer(&mut color).add_option(&["--color", "-c"], Store, "Fill color");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Fill(region, color, max!(cell_index, 1) - 1)
    })
}

pub fn parse_filter(args: &[String]) -> Result<Operation, String> {
    let mut condition = entry_filter::Condition::default();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut condition.min_width).add_option(&["--min-width", "-w"], StoreOption, "Minimum width");
        ap.refer(&mut condition.min_height).add_option(&["--min-height", "-h"], StoreOption, "Minimum height");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Filter(condition.optionize())
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
                InputType::Event =>
                    Ok(Input::Event(o!(text))),
            }
        }
    }

    let mut input_type = InputType::Key;
    let mut input = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut input_type)
            .add_option(&["--key", "-k"], StoreConst(InputType::Key), "Keyboard")
            .add_option(&["--mouse-button", "-m"], StoreConst(InputType::MouseButton), "Mouse button")
            .add_option(&["--event", "-e"], StoreConst(InputType::Event), "Event");
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
    let mut file: String = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut file).add_argument("file-path", Store, "File path").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Load(sh::expand_to_pathbuf(&file))
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
        let mut region: Option<Region> = None;

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target button").required();
            ap.refer(&mut region).add_option(&["--region", "-r"], StoreOption, "Region");
            ap.refer(&mut to).add_argument("to", List, "Command").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::Map(MappingTarget::Mouse(from, region), to)
        })
    }

    fn parse_map_event(args: &[String]) -> Result<Operation, String> {
        let mut event_name = o!("");
        let mut group: Option<String> = None;
        let mut to: Vec<String> = vec![];

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut group).add_option(&["--group", "-g"], StoreOption, "Event group");
            ap.refer(&mut event_name).add_argument("event-name", Store, "Event name").required();
            ap.refer(&mut to).add_argument("to", List, "Command").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::Map(MappingTarget::Event(event_name, group), to)
        })
    }

    fn parse_map_region(args: &[String]) -> Result<Operation, String> {
        let mut from = 1;
        let mut to = vec![];
        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target mouse button").required();
            ap.refer(&mut to).add_argument("to", List, "Operation").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::Map(MappingTarget::Region(from), to)
        })
    }
    if let Some(target) = args.get(1) {
        let args = &args[1..];
        match &**target {
            "k" | "key" => parse_map_key(args),
            "m" | "button" | "mouse" | "mouse-button" => parse_map_mouse(args),
            "e" | "event" => parse_map_event(args),
            "r" | "region" => parse_map_region(args),
            _ => Err(format!("Invalid mapping target: {}", target))
        }
    } else {
        Err(o!("Not enough arguments"))
    }
}

pub fn parse_move_entry(args: &[String]) -> Result<Operation, String> {
    use self::entry::Position::*;

    impl FromStr for entry::Position {
        type Err = String;

        fn from_str(src: &str) -> Result<Self, String> {
            match src {
                "first" => Ok(FromFirst(0)),
                "current" => Ok(Current),
                "last" => Ok(FromLast(0)),
                src =>
                src.parse().map(|n: i64| {
                    if n == 0 {
                        Current
                    } else if n < 0 {
                        FromLast(n.abs() as usize - 1)
                    } else {
                        FromFirst(n.abs() as usize - 1)
                    }
                }).map_err(|it| s!(it)),
            }
        }
    }

    if args.len() != 3 {
        return Err(format!("Invalid number of arguments: {}", args.len() - 1));
    }

    args[1].parse().and_then(|from: entry::Position| {
        args[2].parse().map(|to: entry::Position| {
            Operation::MoveEntry(from, to)
        })
    })
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
where T: FnOnce(String, Option<Meta>, bool) -> Operation {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut path: String = o!("");
    let mut force = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut path).add_argument("Path", Store, "Path to resource").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        op(path, new_opt_meta(meta), force)
    })
}

pub fn parse_push_image(args: &[String]) -> Result<Operation, String> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut path: String = o!("");
    let mut expand_level = None;
    let mut force = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut expand_level)
            .add_option(&["--expand", "-e"], StoreConst(Some(0)), "Push and expand")
            .add_option(&["--expand-recursive", "-E"], StoreOption, "Push and expand recursive");
        ap.refer(&mut path).add_argument("Path", Store, "Path to resource").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::PushImage(Expandable(path), new_opt_meta(meta), force, expand_level)
    })
}

pub fn parse_push_sibling(args: &[String], next: bool) -> Result<Operation, String> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut force = false;
    let mut show = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show the found entry");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::PushSibling(next, new_opt_meta(meta), force, show)
    })
}

pub fn parse_save(args: &[String]) -> Result<Operation, String> {
    let mut path: Option<String> = None;
    let mut sources: Vec<Session> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut sources).add_option(&["--target", "-t"], Collect, "Target");
        ap.refer(&mut path).add_argument("path", StoreOption, "Save to");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if sources.is_empty() {
            sources.push(Session::All);
        }
        Ok(Operation::Save(path.map(|it| sh::expand_to_pathbuf(&it)), sources))
    })
}

pub fn parse_set_env(args: &[String]) -> Result<Operation, String> {
    use constant::*;

    let mut name = o!("");
    let mut value: Option<String> = None;
    let mut prefix: &str = "";

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("env-name", Store, "Env name").required();
        ap.refer(&mut value).add_argument("env-value", StoreOption, "Value");
        ap.refer(&mut prefix)
            .add_option(&["--prefix", "-p"], StoreConst(USER_VARIABLE_PREFIX), "Insert the user prefix `CHRY_X_` to env name")
            .add_option(&["--system-prefix", "-P"], StoreConst(VARIABLE_PREFIX), "Insert the system prefix `CHRY_` to env name");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::SetEnv(format!("{}{}", prefix, name), value.map(Expandable))
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
    let mut sessions: Vec<Session> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async)
            .add_option(&["--async", "-a"], StoreTrue, "Async (Non-blocking)")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync (Blocking)");
        ap.refer(&mut sessions).add_option(&["--session", "-S"], Collect, "Sessions");
        ap.refer(&mut read_operations).add_option(&["--operation", "-o"], StoreTrue, "Read operations form stdout");
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let command_line = command_line.into_iter().map(Expandable).collect();
        Ok(Operation::Shell(async, read_operations, command_line, sessions))
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
    let mut interval_seconds = 1.0;
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
        Operation::Timer(name, op, Duration::from_millis((interval_seconds * 1000.0) as u64), repeat)
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


impl FromStr for Session {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        match src {
            "options" | "option" | "o" =>
                Ok(Session::Options),
            "entries" | "entry" | "e" =>
                Ok(Session::Entries),
            "paths" | "path" | "P" =>
                Ok(Session::Paths),
            "position" | "pos" | "p" =>
                Ok(Session::Position),
            "mappings" | "map" | "m" =>
                Ok(Session::Mappings),
            "envs" | "env" | "E" =>
                Ok(Session::Envs),
            "all" | "a" =>
                Ok(Session::All),
            _ =>
                Err(format!("Invalid stdin source: {}", src))
        }
    }
}


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
