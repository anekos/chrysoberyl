
use std::io::sink;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use argparse::{ArgumentParser, Collect, Store, StoreConst, StoreTrue, StoreFalse, StoreOption, List};

use cherenkov::fill::Shape;
use color::Color;
use entry::filter::expression::Expr as FilterExpr;
use entry::{Meta, MetaEntry, SearchKey, new_opt_meta, EntryType};
use expandable::Expandable;
use key::{Key, new_key_sequence, Coord};
use mapping::{Input, InputType};
use shellexpand_wrapper as sh;
use util::string::join;

use operation::*;
use operation::option::*;



const SEARCH_PATH_DESC: &str = "Search script path from ~/.config/chrysoberyl and /usr/share/chrysoberyl";


pub fn parse_usize<T>(args: &[String], op: T, mut delta: usize) -> Result<Operation, ParsingError>
where T: FnOnce(usize) -> OptionUpdater {
    let mut option_name = OptionName::default();
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut option_name).add_argument("option_name", Store, "Option name").required();
        ap.refer(&mut delta).add_argument("delta", Store, "Delta");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::UpdateOption(option_name, op(delta))
    })
}

pub fn parse_command1<T, U>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: FnOnce(U) -> Operation, U: FromStr {
    if let Some(arg) = args.get(1) {
        U::from_str(arg).map(op).map_err(|_| ParsingError::InvalidArgument(o!(arg)))
    } else {
        Err(ParsingError::TooFewArguments)
    }
}

pub fn parse_move<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
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

pub fn parse_cherenkov(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_file(args: &[String]) -> Result<Operation, ParsingError> {
    use filer::{IfExist, FileOperation};
    use size::Size;

    fn parse<T>(args: &[String], op: T) -> Result<Operation, ParsingError> where T: FnOnce(PathBuf, Option<String>, IfExist, Option<Size>) -> FileOperation {
        let mut destination = "".to_owned();
        let mut filename: Option<String> = None;
        let mut if_exist = IfExist::NewFileName;
        let mut size = None;

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut if_exist)
                .add_option(&["--fail", "-f"], StoreConst(IfExist::Fail), "Fail if file exists")
                .add_option(&["--overwrite", "-o"], StoreConst(IfExist::Overwrite), "Overwrite the file if file exists")
                .add_option(&["--new", "--new-file-name", "-n"], StoreConst(IfExist::NewFileName), "Generate new file name if file exists (default)");
            ap.refer(&mut size)
                .add_option(&["--size", "-s"], StoreOption, "Fit to this size (only for PDF)");
            ap.refer(&mut destination).add_argument("destination", Store, "Destination directory").required();
            ap.refer(&mut filename).add_argument("filename", StoreOption, "Filename");
            parse_args(&mut ap, args)
        } .map(|_| {
            Operation::OperateFile(
                op(sh::expand_to_pathbuf(&destination), filename, if_exist, size))
        })
    }

    if_let_some!(op = args.get(1), Err(ParsingError::TooFewArguments));
    let args = &args[1..];
    let op = match &**op {
        "copy" => FileOperation::new_copy,
        "move" => FileOperation::new_move,
        _ => return Err(ParsingError::InvalidArgument(format!("Invalid file operation: {}", op)))
    };
    parse(args, op)
}

pub fn parse_clip(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_controller(args: &[String]) -> Result<Operation, ParsingError> {
    use controller::Source;

    fn parse_1<T>(args: &[String], f: T) -> Result<Source, ParsingError> where T: FnOnce(Expandable) -> Source {
        let mut path = o!("");

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut path).add_argument("path", Store, "History file").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            f(Expandable::new(path))
        })
    }

    fn parse_2<T>(args: &[String], f: T) -> Result<Source, ParsingError> where T: FnOnce(Expandable, bool) -> Source {
        let mut path = o!("");
        let mut as_binary = false;

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut as_binary).add_option(&["--as-binary", "--as-bin", "-b"], StoreTrue, "As image file");
            ap.refer(&mut path).add_argument("path", Store, "Path").required();
            parse_args(&mut ap, args)
        } .map(|_| {
            f(Expandable::new(path), as_binary)
        })
    }

    if let Some(target) = args.get(1) {
        let args = &args[1..];
        let source = match &**target {
            "fifo" => parse_1(args, Source::Fifo),
            "file" => parse_1(args, Source::File),
            "socket" | "unix-socket" | "sock" => parse_2(args, Source::UnixSocket),
            _ => return Err(ParsingError::InvalidArgument(format!("Invalid controller source: {}", target)))
        };
        source.map(Operation::Controller)
    } else {
        Err(ParsingError::TooFewArguments)
    }
}

pub fn parse_copy_to_clipboard(args: &[String]) -> Result<Operation, ParsingError> {
    let mut selection = ClipboardSelection::Clipboard;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut selection)
            .add_option(&["--clipboard", "-c"], StoreConst(ClipboardSelection::Clipboard), "Use `Clipboard`")
            .add_option(&["--primary", "-1", "-p"], StoreConst(ClipboardSelection::Primary), "Use `Primary`")
            .add_option(&["--secondary", "-2", "-s"], StoreConst(ClipboardSelection::Secondary), "Use `Secondary`");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::CopyToClipboard(selection)
    })
}

pub fn parse_count(args: &[String]) -> Result<Operation, ParsingError> {
    let mut count: Option<usize> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut count).add_argument("count", StoreOption, "Put count");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Count(count)
    })
}

pub fn parse_define_switch(args: &[String]) -> Result<Operation, ParsingError> {
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
            if it == "@@" {
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

pub fn parse_delete(args: &[String]) -> Result<Operation, ParsingError> {
    let mut expr = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut expr).add_argument("expression", Collect, "Filter expression");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let op = join(&expr, ' ').parse().map(|it: FilterExpr| Operation::Delete(Box::new(it.apply_not())));
        let op = op.map_err(ParsingError::InvalidArgument);
        op.map(|op| Operation::WithMessage(Some(o!("Deleting")), Box::new(op)))
    })
}

pub fn parse_editor(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_expand(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_fill(args: &[String]) -> Result<Operation, ParsingError> {
    impl FromStr for Shape {
        type Err = ParsingError;

        fn from_str(src: &str) -> Result<Self, ParsingError> {
            use cherenkov::fill::Shape::*;

            match src {
                "rectangle" | "rect" | "r" => Ok(Rectangle),
                "circle" | "c" => Ok(Circle),
                "ellipse" | "e" => Ok(Ellipse),
                _ => Err(ParsingError::InvalidArgument(format!("Invalid shape: {}", src))),
            }
        }
    }

    let mut cell_index = 1;
    let mut region = None;
    let mut color = Color::black();
    let mut mask = false;
    let mut shape = Shape::Rectangle;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut cell_index).add_option(&["--cell-index", "-i"], Store, "Cell index (1 origin, default = 1)");
        ap.refer(&mut region).add_option(&["--region", "-r"], StoreOption, "Fill target region");
        ap.refer(&mut color).add_option(&["--color", "-c"], Store, "Fill color");
        ap.refer(&mut mask).add_option(&["--mask", "-m"], StoreTrue, "Mask");
        ap.refer(&mut shape).add_option(&["--shape", "-s"], Store, "Shape (rectangle/circle/ellipse)");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Fill(shape, region, color, mask, max!(cell_index, 1) - 1)
    })
}

pub fn parse_filter(args: &[String]) -> Result<Operation, ParsingError> {
    let mut expr = vec![];
    let mut dynamic = true;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut dynamic)
            .add_option(&["--dynamic", "-d"], StoreTrue, "Update dynamic filter (default)")
            .add_option(&["--static", "-s"], StoreFalse, "Update static filter");
        ap.refer(&mut expr).add_argument("expression", Collect, "Filter expression");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let op = if expr.is_empty() {
            Ok(Operation::Filter(dynamic, Box::new(None)))
        } else {
            join(&expr, ' ').parse().map(|it| Operation::Filter(dynamic, Box::new(Some(it))))
        };
        let op = op.map_err(ParsingError::InvalidArgument);
        op.map(|op| Operation::WithMessage(Some(o!("Filtering")), Box::new(op)))
    })
}

pub fn parse_fly_leaves(args: &[String]) -> Result<Operation, ParsingError> {
    let mut n = 0;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut n).add_argument("fly-leaves", Store, "Number of fly-leaves");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::FlyLeaves(n)
    })
}

pub fn parse_go(args: &[String]) -> Result<Operation, ParsingError> {
    let mut key = SearchKey { path: o!(""), index: None };

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut key.path).add_argument("path", Store, "File path / URL");
        ap.refer(&mut key.index).add_argument("page", StoreOption, "Page");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if let Some(index) = key.index.as_mut() {
            if *index == 0 {
                return Err(ParsingError::Fixed("Page is 1 origin"))
            }
            *index -= 1;
        }
        Ok(Operation::Go(key))
    })
}


pub fn parse_input(args: &[String]) -> Result<Operation, ParsingError> {
    impl InputType {
        pub fn input_from_text(&self, text: &str) -> Result<Input, ParsingError> {
            match *self {
                InputType::Unified =>
                    Ok(Input::Unified(Coord::default(), Key(o!(text)))),
                InputType::Event => {
                    match text.parse() {
                        Ok(event) => Ok(Input::Event(event)),
                        Err(err) => Err(ParsingError::InvalidArgument(o!(err))),
                    }
                }
            }
        }
    }

    let mut input_type = InputType::Unified;
    let mut input = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut input_type)
            .add_option(&["--event", "-e"], StoreConst(InputType::Event), "Event");
        ap.refer(&mut input).add_argument("input", Store, "Input").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        input_type.input_from_text(&input).map(|input| {
            Operation::Input(input)
        })
    })
}

pub fn parse_jump(args: &[String]) -> Result<Operation, ParsingError> {
    let mut name = Expandable::default();
    let mut load = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("name", Store, "Marker name").required();
        ap.refer(&mut load).add_option(&["--load", "-l"], StoreTrue, "Load if the target has not been loaded");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Jump(name, load)
    })
}

pub fn parse_kill_timer(args: &[String]) -> Result<Operation, ParsingError> {
    let mut name = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("name", Store, "Name").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::KillTimer(name)
    })
}

pub fn parse_load(args: &[String]) -> Result<Operation, ParsingError> {
    let mut file: String = o!("");
    let mut search_path = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut file).add_argument("file-path", Store, "File path").required();
        ap.refer(&mut search_path).add_option(&["--search-path", "-p"], StoreTrue, SEARCH_PATH_DESC);
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Load(Expandable::new(file), search_path)
    })
}

pub fn parse_map(args: &[String], register: bool) -> Result<Operation, ParsingError> {
    fn parse_map_unified(args: &[String], register: bool) -> Result<Operation, ParsingError> {
        let mut from = "".to_owned();
        let mut to: Vec<String> = vec![];
        let mut region: Option<Region> = None;

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target input sequence").required();
            ap.refer(&mut region).add_option(&["--region", "-r"], StoreOption, "Region");
            if register {
                ap.refer(&mut to).add_argument("to", List, "Command").required();
            }
            parse_args(&mut ap, args)
        } .map(|_| {
            let target = MappingTarget::Unified(new_key_sequence(&from), region);
            if register {
                Operation::Map(target, None, to)
            } else {
                Operation::Unmap(target)
            }
        })
    }

    fn parse_map_event(args: &[String], register: bool) -> Result<Operation, ParsingError> {
        let mut event_name = None;
        let mut group: Option<String> = None;
        let mut to: Vec<String> = vec![];
        let mut remain = None;

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut group).add_option(&["--group", "-g"], StoreOption, "Event group");
            ap.refer(&mut remain)
                .add_option(&["--once", "-o", "-1"], StoreConst(Some(1)), "Once")
                .add_option(&["--repeat", "-r"], StoreOption, "Repeat count");
            {
                let mut en = ap.refer(&mut event_name);
                en.add_argument("event-name", StoreOption, "Event name");
                if register {
                    en.required();
                }
            }
            if register {
                ap.refer(&mut to).add_argument("to", List, "Command").required();
            }
            parse_args(&mut ap, args)
        } .map(|_| {
            let target = MappingTarget::Event(event_name, group);
            if register {
                Operation::Map(target, remain, to)
            } else {
                Operation::Unmap(target)
            }
        })
    }

    fn parse_map_region(args: &[String], register: bool) -> Result<Operation, ParsingError> {
        let mut from = "".to_owned();
        let mut to = vec![];
        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut from).add_argument("from", Store, "Target mouse button").required();
            if register {
                ap.refer(&mut to).add_argument("to", List, "Operation").required();
            }
            parse_args(&mut ap, args)
        } .map(|_| {
            let target = MappingTarget::Region(Key(from));
            if register {
                Operation::Map(target, None, to)
            } else {
                Operation::Unmap(target)
            }
        })
    }

    if let Some(target) = args.get(1) {
        let args = &args[1..];
        match &**target {
            "i" | "input" => parse_map_unified(args, register),
            "e" | "event" => parse_map_event(args, register),
            "r" | "region" => parse_map_region(args, register),
            _ => Err(ParsingError::InvalidArgument(format!("Invalid mapping target: {}", target)))
        }
    } else {
        Err(ParsingError::TooFewArguments)
    }
}

pub fn parse_mark(args: &[String]) -> Result<Operation, ParsingError> {
    let mut name = Expandable::default();
    let mut path = None;
    let mut index: Option<usize> = None;
    let mut entry_type = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut name).add_argument("name", Store, "Marker name").required();
        ap.refer(&mut path).add_argument("path", StoreOption, "Path");
        ap.refer(&mut index).add_argument("index", StoreOption, "Index");
        ap.refer(&mut entry_type).add_argument("entry-type", StoreOption, "Entry type");
        parse_args(&mut ap, args)
    } .map(|_| {
        let index = index.and_then(|it| it.checked_sub(1)).unwrap_or(0);
        Operation::Mark(
            name,
            path.map(|path| (path, index, entry_type)))
    })
}

pub fn parse_message(args: &[String]) -> Result<Operation, ParsingError> {
    let mut message = None;
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut message).add_argument("message", StoreOption, "Message");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Message(message)
    })
}

pub fn parse_multi(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_multi_args(xs: &[String], separator: &str, async: bool) -> Result<Operation, ParsingError> {
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
        match _parse_from_vec(&op) {
            Ok(op) => result.push_back(op),
            err => return err
        }
    }

    Ok(Operation::Multi(result, async))
}

pub fn parse_operation_entry(args: &[String]) -> Result<Operation, ParsingError> {
    let mut action = OperationEntryAction::Open;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut action).add_argument("action", Store, "Action");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::OperationEntry(action)
    })
}

pub fn parse_option_cycle(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_option_set(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_option_1(args: &[String], updater: OptionUpdater) -> Result<Operation, ParsingError> {
    let mut option_name = OptionName::default();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut option_name).add_argument("option_name", Store, "Option name").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::UpdateOption(option_name, updater)
    })
}

pub fn parse_page(args: &[String]) -> Result<Operation, ParsingError> {
    let mut page = 1;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut page).add_argument("page", Store, "Page number");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        Ok(Operation::Page(page))
    })
}

pub fn parse_pdf_index(args: &[String]) -> Result<Operation, ParsingError> {
    use poppler::index::Format;

    let mut async = true;
    let mut read_operations = true;
    let mut search_path = false;
    let mut command_line: Vec<String> = vec![];
    let mut fmt = Format::default();
    let mut fmt_separator: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async)
            .add_option(&["--async", "-a"], StoreTrue, "Async (Non-blocking)")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync (Blocking)");
        ap.refer(&mut search_path).add_option(&["--search-path", "-p"], StoreTrue, SEARCH_PATH_DESC);
        ap.refer(&mut read_operations)
            .add_option(&["--operation", "-o"], StoreTrue, "Read operations from stdout")
            .add_option(&["--no-operation", "-O"], StoreTrue, "Dont read operations from stdout");
        ap.refer(&mut fmt_separator)
            .add_option(&["--separator"], StoreOption, "Separator for `indented`");
        ap.refer(&mut fmt)
            .add_option(&["--format", "-f"], Store, "Format (1/2/indented)");
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let command_line = command_line.into_iter().map(Expandable::new).collect();
        Ok(Operation::PdfIndex(async, read_operations, search_path, command_line, fmt, fmt_separator))
    })
}

pub fn parse_push<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: Fn(String, Option<Meta>, bool) -> Operation {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut paths = Vec::<String>::new();
    let mut force = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut paths).add_argument("Path", Collect, "Path to resource").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        let ops = paths.into_iter().map(|it| op(it, meta.clone(), force)).collect();
        Operation::Multi(ops, false)
    })
}

pub fn parse_push_clipboard(args: &[String]) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut selection = ClipboardSelection::Clipboard;
    let mut force = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut selection)
            .add_option(&["--clipboard", "-c"], StoreConst(ClipboardSelection::Clipboard), "Use `Clipboard`")
            .add_option(&["--primary", "-1", "-p"], StoreConst(ClipboardSelection::Primary), "Use `Primary`")
            .add_option(&["--secondary", "-2", "-s"], StoreConst(ClipboardSelection::Secondary), "Use `Secondary`");
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        Operation::PushClipboard(selection, meta, force)
    })
}

pub fn parse_push_image(args: &[String]) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut paths = Vec::<String>::new();
    let mut expand_level = None;
    let mut force = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut expand_level)
            .add_option(&["--expand", "-e"], StoreConst(Some(0)), "Push and expand")
            .add_option(&["--expand-recursive", "-E"], StoreOption, "Push and expand recursive");
        ap.refer(&mut paths).add_argument("Path", Collect, "Path to image file").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        let ops = paths.into_iter().map(|it| Operation::PushImage(Expandable::new(it), meta.clone(), force, expand_level)).collect();
        Operation::Multi(ops, false)
    })
}

pub fn parse_push_sibling(args: &[String], next: bool) -> Result<Operation, ParsingError> {
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

pub fn parse_push_url(args: &[String]) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut urls = Vec::<String>::new();
    let mut force = false;
    let mut entry_type = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut entry_type).add_option(&["--type", "-t", "--as"], StoreOption, "Type (image/archive/pdf)");
        ap.refer(&mut urls).add_argument("URL", Collect, "URL").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        let ops = urls.into_iter().map(|it| Operation::PushURL(it, meta.clone(), force, entry_type)).collect();
        Operation::Multi(ops, false)
    })
}

pub fn parse_query(args: &[String]) -> Result<Operation, ParsingError> {
    let mut operation = vec![];
    let mut caption = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut caption).add_option(&["--caption", "-c"], StoreOption, "Caption");
        ap.refer(&mut operation).add_argument("Operation", Collect, "Operation").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Query(operation, caption)
    })
}

pub fn parse_refresh(args: &[String]) -> Result<Operation, ParsingError> {
    let mut image = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut image).add_option(&["--image", "-i"], StoreTrue, "Refresh image cache");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        Ok(Operation::Refresh(image))
    })
}

pub fn parse_save(args: &[String]) -> Result<Operation, ParsingError> {
    let mut path: String = o!("");
    let mut sources: Vec<Session> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut sources).add_option(&["--target", "-t"], Collect, "Target");
        ap.refer(&mut path).add_argument("path", Store, "Save to").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if sources.is_empty() {
            sources.push(Session::All);
        }
        Ok(Operation::Save(sh::expand_to_pathbuf(&path), sources))
    })
}

pub fn parse_set_env(args: &[String]) -> Result<Operation, ParsingError> {
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
        Operation::SetEnv(format!("{}{}", prefix, name), value.map(Expandable::new))
    })
}

pub fn parse_scroll(args: &[String]) -> Result<Operation, ParsingError> {
    let mut direction = Direction::Up;
    let mut operation = vec![];
    let mut scroll_size = 1.0;
    let mut crush = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut direction).add_argument("direction", Store, "left|up|right|down").required();
        ap.refer(&mut scroll_size).add_option(&["-s", "--size"], Store, "Scroll size (default 1.0) ");
        ap.refer(&mut crush).add_option(&["-c", "--crush"], StoreTrue, "Crush a little space");
        ap.refer(&mut operation).add_argument("operation", List, "Operation");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Scroll(direction, operation, scroll_size, crush)
    })
}

pub fn parse_search(args: &[String]) -> Result<Operation, ParsingError> {
    let mut text = None;
    let mut backward = false;
    let mut color = Color::new4(255, 255, 0, 128);

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut text).add_argument("text", StoreOption, "Search text");
        ap.refer(&mut backward).add_option(&["-b", "--backward"], StoreTrue, "Search backward");
        ap.refer(&mut color).add_option(&["-c", "--color"], Store, "Highlight color");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::WithMessage(
            Some(o!("Searching")),
            Box::new(Operation::SearchText(text, backward, color)))
    })
}

pub fn parse_shell(args: &[String]) -> Result<Operation, ParsingError> {
    let mut async = true;
    let mut read_operations = false;
    let mut search_path = false;
    let mut command_line: Vec<String> = vec![];
    let mut sessions: Vec<Session> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut async)
            .add_option(&["--async", "-a"], StoreTrue, "Async (Non-blocking)")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync (Blocking)");
        ap.refer(&mut sessions).add_option(&["--session", "-S"], Collect, "Sessions");
        ap.refer(&mut read_operations)
            .add_option(&["--operation", "-o"], StoreTrue, "Read operations from stdout")
            .add_option(&["--no-operation", "-O"], StoreTrue, "Dont read operations from stdout");
        ap.refer(&mut search_path).add_option(&["--search-path", "-p"], StoreTrue, SEARCH_PATH_DESC);
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let command_line = command_line.into_iter().map(Expandable::new).collect();
        Ok(Operation::Shell(async, read_operations, search_path, command_line, sessions))
    })
}

pub fn parse_shell_filter(args: &[String]) -> Result<Operation, ParsingError> {
    let mut search_path = false;
    let mut command_line: Vec<String> = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut search_path).add_option(&["--search-path", "-p"], StoreTrue, SEARCH_PATH_DESC);
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let command_line = command_line.into_iter().map(Expandable::new).collect();
        Ok(Operation::ShellFilter(command_line, search_path))
    })
}

pub fn parse_sort(args: &[String]) -> Result<Operation, ParsingError> {
    let mut fix = false;
    let mut sort_key = SortKey::Natural;
    let mut reverse = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut fix).add_option(&["--fix", "-f"], StoreTrue, "Fix current page");
        ap.refer(&mut sort_key)
            .add_option(&["--accessed", "-a"], StoreConst(SortKey::Accessed), "Sort by accessed time")
            .add_option(&["--created", "-c"], StoreConst(SortKey::Created), "Sort by created time")
            .add_option(&["--modified", "-m"], StoreConst(SortKey::Modified), "Sort by modified time")
            .add_option(&["--file-size", "-s"], StoreConst(SortKey::FileSize), "Sort by file size")
            .add_option(&["--width", "-w"], StoreConst(SortKey::Width), "Sort by width")
            .add_option(&["--height", "-h"], StoreConst(SortKey::Height), "Sort by heigth")
            .add_option(&["--dimensions", "-d"], StoreConst(SortKey::Dimensions), "Sort by width x height");
        ap.refer(&mut reverse).add_option(&["--reverse", "-r"], StoreTrue, "Reversed");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::WithMessage(
            Some(o!("Sorting")),
            Box::new(Operation::Sort(fix, sort_key, reverse)))
    })
}

pub fn parse_modify_entry_order<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: FnOnce(bool) -> Operation {
    let mut fix = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut fix)
            .add_option(&["--fix", "-f"], StoreTrue, "Fix current page");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        Ok(op(fix))
    })
}

pub fn parse_timer(args: &[String]) -> Result<Operation, ParsingError> {
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
        ap.refer(&mut interval_seconds).add_argument("interval", Store, "Interval").required();
        ap.refer(&mut op).add_argument("operation", Collect, "Operation").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Timer(name, op, Duration::from_millis((interval_seconds * 1000.0) as u64), repeat)
    })
}

pub fn parse_undo(args: &[String]) -> Result<Operation, ParsingError> {
    let mut count = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut count).add_argument("Count", StoreOption, "Count");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Undo(count)
    })
}

pub fn parse_update(args: &[String]) -> Result<Operation, ParsingError> {
    let mut updated = Updated::default();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut updated.image).add_option(&["--image", "-i"], StoreTrue, "Update image");
        ap.refer(&mut updated.image_options).add_option(&["--image-options", "-o"], StoreTrue, "Update image_options");
        ap.refer(&mut updated.label).add_option(&["--label", "-l"], StoreTrue, "Update label");
        ap.refer(&mut updated.message).add_option(&["--message", "-m"], StoreTrue, "Update message");
        ap.refer(&mut updated.pointer).add_option(&["--pointer", "-p"], StoreTrue, "Update pointer");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Update(updated)
    })
}

pub fn parse_views(args: &[String]) -> Result<Operation, ParsingError> {
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
            return Err(ParsingError::Fixed("Columns / rows must be greater than 0"))
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

pub fn parse_when(args: &[String], unless: bool) -> Result<Operation, ParsingError> {
    let mut op = Vec::<String>::new();
    let mut filter = FilterExpr::default();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut filter).add_argument("filter", Store, "Filter expression").required();
        ap.refer(&mut op).add_argument("operation", Collect, "Operation").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::When(filter, unless, op)
    })
}

pub fn parse_write(args: &[String]) -> Result<Operation, ParsingError> {
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

pub fn parse_args(parser: &mut ArgumentParser, args: &[String]) -> Result<(), ParsingError> {
    parser.stop_on_first_argument(true);
    parser.parse(args.to_vec(), &mut sink(), &mut sink()).map_err(|code| ParsingError::InvalidArgument(s!(code)))
}


impl FromStr for Session {
    type Err = ParsingError;

    fn from_str(src: &str) -> Result<Self, ParsingError> {
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
            "filter" | "f" =>
                Ok(Session::Filter),
            "reading" | "read" | "r" =>
                Ok(Session::Reading),
            "markers" | "marks" =>
                Ok(Session::Markers),
            "all" | "a" =>
                Ok(Session::All),
            _ =>
                Err(ParsingError::InvalidArgument(format!("Invalid stdin source: {}", src)))
        }
    }
}


impl FromStr for MetaEntry {
    type Err = ParsingError;

    fn from_str(src: &str) -> Result<MetaEntry, ParsingError> {
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


impl FromStr for EntryType {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        match src {
            "image" | "img" | "o" =>
                Ok(EntryType::Image),
            "archive" | "arc" | "a" =>
                Ok(EntryType::Archive),
            "pdf" | "p" | "portable-document-format" =>
                Ok(EntryType::PDF),
            _ =>
                Err(format!("Invalid type: {}", src))
        }
    }
}

impl FromStr for OperationEntryAction {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::OperationEntryAction::*;

        match src {
            "open" | "o" =>
                Ok(Open),
            "close" | "c" =>
                Ok(Close),
            "send" | "s" =>
                Ok(SendOperation),
            _ =>
                Err(format!("Invalid action: {}", src))
        }
    }
}
