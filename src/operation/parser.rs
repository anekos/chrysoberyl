
use std::io::sink;
use std::path::PathBuf;
use std::str::FromStr;

use argparse::{ArgumentParser, Collect, Store, StoreConst, StoreTrue, StoreOption, List, PushConst};
use css_color_parser::Color as CssColor;

use config::ConfigSource;
use entry::{Meta, MetaEntry, new_meta_from_vec};
use filer;
use gui::ColorTarget;
use mapping::{InputType, mouse_mapping};

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
where T: FnOnce(Option<usize>, bool) -> Operation {
    let mut ignore_views = false;
    let mut count = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut ignore_views).add_option(&["--ignore-views", "-i"], StoreTrue, "Ignore the number of views");
        ap.refer(&mut count).add_argument("count", StoreOption, "Count");
        parse_args(&mut ap, args)
    } .map(|_| {
        op(count, ignore_views)
    })
}

pub fn parse_cherenkov(args: &[String]) -> Result<Operation, String> {
    let mut radius = 0.1;
    let mut random_hue = 0.0;
    let mut n_spokes = 50;
    let mut x = None;
    let mut y = None;
    let mut color: CssColor = "blue".parse().unwrap();
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
        (expand_to_pathbuf(&destination).to_owned(), if_exist)
    })
}

pub fn parse_color(args: &[String]) -> Result<Operation, String> {
    let mut target: ColorTarget = ColorTarget::WindowBackground;
    let mut color: CssColor = "white".parse().unwrap();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut target).add_argument("target", Store, "Target").required();
        ap.refer(&mut color).add_argument("color", Store, "CSS Color").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Color(target, color)
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
    let mut command_line: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut config_sources)
            .add_option(&["--user", "-u"], PushConst(ConfigSource::User), "Insert user config")
            .add_option(&["--default", "-d"], PushConst(ConfigSource::Default), "Insert defult config");
        ap.refer(&mut command_line).add_argument("command-line", StoreOption, "Command line to open editor");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Editor(command_line, config_sources)
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
        Operation::Expand(recursive, base.map(|it| pathbuf(&it)))
    })
}

pub fn parse_input(args: &[String]) -> Result<Operation, String> {
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

pub fn parse_load(args: &[String]) -> Result<Operation, String> {
    let mut config_source = ConfigSource::Default;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut config_source)
            .add_option(&["--user", "-u"], StoreConst(ConfigSource::User), "Load user config (rc.conf)")
            .add_option(&["--default", "-d"], StoreConst(ConfigSource::Default), "Load default config");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::LoadConfig(config_source)
    })
}

pub fn parse_map(args: &[String]) -> Result<Operation, String> {
    fn parse_map_key(args: &[String]) -> Result<Operation, String> {
        let mut from = "".to_owned();
        let mut to: Vec<String> = vec![];
        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target key name").required();
            ap.refer(&mut to).add_argument("to", List, "Command").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::Map(MappingTarget::Key(from), to)
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

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut separator).add_argument("separator", Store, "Commands separator").required();
        ap.refer(&mut commands).add_argument("arguments", List, "Commands");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        parse_multi_args(&commands, &separator)
    })
}

pub fn parse_multi_args(xs: &[String], separator: &str) -> Result<Operation, String> {
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

    let mut result: Vec<Operation> = vec![];

    for op in ops {
        match parse_from_vec(&op) {
            Ok(op) => result.push(op),
            err => return err
        }
    }

    Ok(Operation::Multi(result))
}

pub fn parse_option_updater(args: &[String], modifier: StateUpdater) -> Result<Operation, String> {
    use state::StateName::*;
    use self::Operation::UpdateOption;

    let mut name = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("option_name", Store, "Option name").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        match &*name.to_lowercase() {
            "status-bar" | "status" => Ok(UpdateOption(StatusBar, modifier)),
            "reverse" | "rev" => Ok(UpdateOption(Reverse, modifier)),
            "center" | "center-alignment" => Ok(UpdateOption(CenterAlignment, modifier)),
            "fit" => Ok(UpdateOption(Fit, modifier)),
            _  => Err(format!("Unknown option: {}", name))
        }
    })
}

pub fn parse_push<T>(args: &[String], op: T) -> Result<Operation, String>
where T: FnOnce(String, Meta) -> Operation {
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
        op(path, new_meta_from_vec(meta))
    })
}

pub fn parse_save(args: &[String]) -> Result<Operation, String> {
    let mut index = None;
    let mut path = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut index).add_option(&["--index", "-i"], StoreOption, "Index (1 origin)");
        ap.refer(&mut path).add_argument("path", Store, "Save to").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Save(expand_to_pathbuf(&path), index)
    })
}

pub fn parse_scaling(args: &[String]) -> Result<Operation, String> {
    let mut scaling_method = ScalingMethod::default();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut scaling_method).add_argument("method-name", Store, "Scaling method (nearest/tiles/bilinear/hyper)").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::ChangeScalingMethod(scaling_method)
    })
}

pub fn parse_shell(args: &[String]) -> Result<Operation, String> {
    let mut async = false;
    let mut read_operations = false;
    let mut command_line: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async).add_option(&["--async", "-a"], StoreTrue, "Async");
        ap.refer(&mut read_operations).add_option(&["--operation", "-o"], StoreTrue, "Read operations form stdout");
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        parse_args(&mut ap, args)
    } .map(|_| {
        let command_line = command_line.iter().map(|it| expand(it)).collect();
        Operation::Shell(async, read_operations, command_line)
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

pub fn parse_args(parser: &mut ArgumentParser, args: &[String]) -> Result<(), String> {
    parser.stop_on_first_argument(true);
    parser.parse(args.to_vec(), &mut sink(), &mut sink()).map_err(|code| s!(code))
}
