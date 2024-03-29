
use std::io::sink;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

use argparse::{ArgumentParser, Collect, Store, StoreConst, StoreTrue, StoreFalse, StoreOption, List};

use crate::chainer;
use crate::cherenkov::fill::Shape;
use crate::cherenkov::nova::Seed;
use crate::color::Color;
use crate::entry::filter::expression::Expr as FilterExpr;
use crate::entry::{Meta, MetaEntry, SearchKey, new_opt_meta};
use crate::expandable::Expandable;
use crate::filer::{IfExist, FileOperation};
use crate::key::{Key, new_key_sequence};
use crate::mapping::{Mapped, MappedType};
use crate::shellexpand_wrapper as sh;
use crate::size::{CoordPx, Size};
use crate::util::string::join;

use crate::operation::*;
use crate::operation::option::*;



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

pub fn parse_chainer<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: FnOnce(chainer::Target) -> Operation {
    let mut is_file = false;
    let mut target = o!("");
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut is_file)
            .add_option(&["-f", "--file"], StoreTrue, "Target is file")
            .add_option(&["-p", "--process"], StoreFalse, "Target is process (default)");
        ap.refer(&mut target).add_argument("PID/Path", Store, "PID or File path");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        let target = if is_file {
            chainer::Target::File(sh::expand_to_pathbuf(&target))
        } else {
            chainer::Target::Process(target.parse().map_err(|_| ParsingError::InvalidArgument(target))?)
        };
        Ok(op(target))
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

pub fn parse_common_move<'a, 'b>(ap: &'a mut ArgumentParser<'b>, count: &'b mut Option<usize>, ignore_views: &'b mut bool, move_by: &'b mut MoveBy, wrap: &'b mut bool) {
    ap.refer(ignore_views).add_option(&["--ignore-views", "-i"], StoreTrue, "Ignore the number of views");
    ap.refer(wrap).add_option(&["--wrap", "-w"], StoreTrue, "First/Last page to Last/First page");
    ap.refer(move_by).add_option(&["--archive", "-a"], StoreConst(MoveBy::Archive), "Set move unit to `archive`");
    ap.refer(count).add_argument("count", StoreOption, "Count");
}

pub fn parse_move<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: FnOnce(Option<usize>, bool, MoveBy, bool) -> Operation {
    let mut ignore_views = false;
    let mut count = None;
    let mut move_by = MoveBy::Page;
    let mut wrap = false;

    {
        let mut ap = ArgumentParser::new();
        parse_common_move(&mut ap, &mut count, &mut ignore_views, &mut move_by, &mut wrap);
        parse_args(&mut ap, args)
    } .map(|_| {
        op(count, ignore_views, move_by, wrap)
    })
}

pub fn parse_move5<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: FnOnce(Option<usize>, bool, MoveBy, bool, bool) -> Operation {
    let mut ignore_views = false;
    let mut count = None;
    let mut move_by = MoveBy::Page;
    let mut wrap = false;
    let mut remember = true;

    {
        let mut ap = ArgumentParser::new();
        parse_common_move(&mut ap, &mut count, &mut ignore_views, &mut move_by, &mut wrap);
        ap.refer(&mut remember).add_option(&["--forget", "-f"], StoreFalse, "Don't remember the direction for `@move-again`");
        parse_args(&mut ap, args)
    } .map(|_| {
        op(count, ignore_views, move_by, wrap, remember)
    })
}

pub fn parse_move_again(args: &[String]) -> Result<Operation, ParsingError> {
    let mut ignore_views = false;
    let mut count = None;
    let mut move_by = MoveBy::Page;
    let mut wrap = false;
    let mut reverse = false;

    {
        let mut ap = ArgumentParser::new();
        parse_common_move(&mut ap, &mut count, &mut ignore_views, &mut move_by, &mut wrap);
        ap.refer(&mut reverse).add_option(&["--reverse", "-r"], StoreTrue, "Reverse direction");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::MoveAgain(count, ignore_views, move_by, wrap, reverse)
    })
}

pub fn parse_apng(args: &[String]) -> Result<Operation, ParsingError> {
    let mut path: String = o!("");
    let mut length = 4;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut length).add_option(&["--length", "-l"], Store, "Animation length");
        ap.refer(&mut path).add_argument("path", Store, "Save to").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Apng(sh::expand_to_pathbuf(&path), length)
    })
}

pub fn parse_cherenkov(args: &[String]) -> Result<Operation, ParsingError> {
    let mut p = CherenkovParameter {
        radius: 0.1,
        random_hue: 0.0,
        n_spokes: 50,
        threads: None,
        x: None,
        y: None,
        color: "random".parse().unwrap(),
        seed: Seed::new(None),
    };
    let mut detect_eyes = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut p.radius).add_option(&["--radius", "-r"], Store, "Radius");
        ap.refer(&mut p.random_hue).add_option(&["--random-hue", "-h", "--hue"], Store, "Random Hue");
        ap.refer(&mut p.n_spokes).add_option(&["--spokes", "-s"], Store, "Number of spokes");
        ap.refer(&mut p.x).add_option(&["-x"], StoreOption, "X");
        ap.refer(&mut p.y).add_option(&["-y"], StoreOption, "Y");
        ap.refer(&mut p.color).add_option(&["-c", "--color"], Store, "CSS Color");
        ap.refer(&mut p.seed).add_option(&["-S", "--seed"], Store, "Seed for random number generator");
        ap.refer(&mut p.threads).add_option(&["-t", "--threads", "--thread"], StoreOption, "Number of threads");
        ap.refer(&mut detect_eyes).add_option(&["-d", "--detect-eyes"], StoreTrue, "Detect eyes");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if p.n_spokes == 0 {
            return Err(ParsingError::InvalidArgument(o!("--spokes must be larger than 0")));
        }
        let op = if detect_eyes {
            Operation::DetectEyes(p)
        } else {
            Operation::Cherenkov(p)
        };
        Ok(Operation::WithMessage(Some(o!("Cherenkoving")), Box::new(op)))
    })
}

pub fn parse_file<F>(args: &[String], op: F) -> Result<Operation, ParsingError>
where F: FnOnce(PathBuf, Option<String>, IfExist, Option<Size>) -> FileOperation {
    let mut destination = "".to_owned();
    let mut filename: Option<String> = None;
    let mut if_exist = IfExist::NewFileName;
    let mut as_filepath = false;
    let mut size = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut if_exist)
            .add_option(&["--fail", "-f"], StoreConst(IfExist::Fail), "Fail if file exists")
            .add_option(&["--overwrite", "-o"], StoreConst(IfExist::Overwrite), "Overwrite the file if file exists")
            .add_option(&["--new", "--new-file-name", "-n"], StoreConst(IfExist::NewFileName), "Generate new file name if file exists (default)");
        ap.refer(&mut size)
            .add_option(&["--size", "-s"], StoreOption, "Fit to this size (only for PDF)");
        ap.refer(&mut as_filepath)
            .add_option(&["--as-filepath", "-F"], StoreTrue, "Destination as filepath");
        ap.refer(&mut destination).add_argument("destination", Store, "Destination directory").required();
        ap.refer(&mut filename).add_argument("filename", StoreOption, "Filename");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if as_filepath {
            if let Some(filename) = filename {
                return Err(ParsingError::InvalidArgument(format!("If `--as-filepath` is given, omit `{}`", filename)));
            }
            let destination = sh::expand_to_pathbuf(&destination);
            if_let_some!(dir = destination.parent(), Err(ParsingError::InvalidArgument(format!("No directory part: {:?}", destination))));
            if_let_some!(filename = destination.file_name(), Err(ParsingError::InvalidArgument(format!("No filename part: {:?}", destination))));
            let filename = o!(filename.to_str().unwrap());
            return Ok(Operation::OperateFile(op(dir.to_path_buf(), Some(filename), if_exist, size)));
        }
        Ok(Operation::OperateFile(op(sh::expand_to_pathbuf(&destination), filename, if_exist, size)))
    })
}

pub fn parse_fire(args: &[String]) -> Result<Operation, ParsingError> {
    impl MappedType {
        pub fn input_from_text(self, text: &str) -> Result<Mapped, ParsingError> {
            match self {
                MappedType::Input =>
                    Ok(Mapped::Input(CoordPx::default(), Key::from(text))),
                MappedType::Event => {
                    match text.parse() {
                        Ok(event) => Ok(Mapped::Event(event)),
                        Err(err) => Err(ParsingError::InvalidArgument(o!(err))),
                    }
                }
            }
        }
    }

    let mut mapped_type = MappedType::Input;
    let mut mapped = "".to_owned();

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut mapped_type).add_argument("MappedType", Store, "Mapped type").required();
        ap.refer(&mut mapped).add_argument("Mapped", Store, "Mapped").required();
        parse_args(&mut ap, args)
    } .and_then(|_| {
        mapped_type.input_from_text(&mapped).map(|mapped| {
            Operation::Fire(mapped)
        })
    })
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

pub fn parse_controller<F>(args: &[String], f: F) -> Result<Operation, ParsingError>
where F: FnOnce(Expandable) -> controller::Source {
    let mut path = o!("");

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut path).add_argument("path", Store, "History file").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Controller(f(Expandable::new(path)))
    })
}

pub fn parse_controller_socket(args: &[String]) -> Result<Operation, ParsingError> {
    use crate::controller::Source;

    let mut path = o!("");
    let mut as_binary = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut as_binary).add_option(&["--binary", "--bin", "-b"], StoreTrue, "As image file");
        ap.refer(&mut path).add_argument("path", Store, "Path").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Controller(Source::UnixSocket(Expandable::new(path), as_binary))
    })
}

pub fn parse_copy_to_clipboard(args: &[String]) -> Result<Operation, ParsingError> {
    let mut selection = ClipboardSelection::default();

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
    } .and_then(|_| {
        if count == Some(0) {
            Err(ParsingError::Fixed("Zero is invalid"))
        } else {
            Ok(Operation::Count(count))
        }
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
    let mut comment_out = false;
    let mut command_line: Vec<Expandable> = vec![];
    let mut freeze = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut files).add_option(&["--file", "-f"], Collect, "Insert the given file");
        ap.refer(&mut sessions).add_option(&["--session", "-S"], Collect, "Sessions");
        ap.refer(&mut comment_out).add_option(&["--comment-out", "-c"], StoreTrue, "Comment out");
        ap.refer(&mut freeze).add_option(&["--freeze", "-F"], StoreTrue, "Insert freezer to stop drawing");
        ap.refer(&mut command_line).add_argument("command-line", Collect, "Command line to open editor");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Editor(command_line, files, sessions, comment_out, freeze)
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
            use crate::cherenkov::fill::Shape::*;

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
    let mut operator = None;
    let mut mask = false;
    let mut shape = Shape::Rectangle;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut cell_index).add_option(&["--cell-index", "-i"], Store, "Cell index (1 origin, default = 1)");
        ap.refer(&mut region).add_option(&["--region", "-r"], StoreOption, "Fill target region");
        ap.refer(&mut color).add_option(&["--color", "-c"], Store, "Fill color");
        ap.refer(&mut mask).add_option(&["--mask", "-m"], StoreTrue, "Mask");
        ap.refer(&mut operator).add_option(&["--operator", "-o"], StoreOption, "Operator");
        ap.refer(&mut shape).add_option(&["--shape", "-s"], Store, "Shape (rectangle/circle/ellipse)");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Fill(shape, region, color, operator, mask, max!(cell_index, 1) - 1)
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

pub fn parse_gif(args: &[String]) -> Result<Operation, ParsingError> {
    let mut path: String = o!("");
    let mut length = 4;
    let mut show = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut length).add_option(&["--length", "-l"], Store, "Animation length");
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show the found entry");
        ap.refer(&mut path).add_argument("path", Store, "Save to").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Gif(sh::expand_to_pathbuf(&path), length, show)
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
    let mut mapped: Vec<Mapped> = vec![];

    for arg in args.iter().skip(1) {
        for it in arg.split(',') {
            mapped.push(Mapped::Input(CoordPx::default(), Key::from(it)));
        }
    }

    Ok(Operation::Input(mapped))
}

pub fn parse_jump(args: &[String]) -> Result<Operation, ParsingError> {
    let mut name = o!("");
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

pub fn parse_load<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: Fn(Expandable, bool) -> Operation {
    let mut file: String = o!("");
    let mut search_path = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut file).add_argument("file-path", Store, "File path").required();
        ap.refer(&mut search_path).add_option(&["--search-path", "-p"], StoreTrue, SEARCH_PATH_DESC);
        parse_args(&mut ap, args)
    } .map(|_| {
        op(Expandable::new(file), search_path)
    })
}

pub fn parse_map(args: &[String], register: bool) -> Result<Operation, ParsingError> {
    fn parse_map_operation(args: &[String], register: bool) -> Result<Operation, ParsingError> {
        let mut name = "".to_owned();
        let mut to: Vec<String> = vec![];

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut name).add_argument("from", Store, "Operation name").required();
            if register {
                ap.refer(&mut to).add_argument("to", List, "Operation").required();
            }
            parse_args(&mut ap, args)
        } .map(|_| {
            let target = MappingTarget::Operation(name);
            if register {
                Operation::Map(target, None, to)
            } else {
                Operation::Unmap(target)
            }
        })
    }

    fn parse_map_input(args: &[String], register: bool) -> Result<Operation, ParsingError> {
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
            let target = MappingTarget::Input(new_key_sequence(&from), region);
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
            let target = MappingTarget::Region(Key::new(from));
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
            "i" | "input" => parse_map_input(args, register),
            "e" | "event" => parse_map_event(args, register),
            "o" | "operation" => parse_map_operation(args, register),
            "r" | "region" => parse_map_region(args, register),
            _ => Err(ParsingError::InvalidArgument(format!("Invalid mapping target: {}", target)))
        }
    } else {
        Err(ParsingError::TooFewArguments)
    }
}

pub fn parse_mark(args: &[String]) -> Result<Operation, ParsingError> {
    let mut name = o!("");
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
    let mut keep = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut message).add_argument("message", StoreOption, "Message");
        ap.refer(&mut keep).add_option(&["--keep", "-k"], StoreTrue, "Keep current message (No overwrite)");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Message(message, keep)
    })
}

pub fn parse_multi(args: &[String]) -> Result<Operation, ParsingError> {
    let mut separator = "".to_owned();
    let mut commands: Vec<String> = vec![];
    let mut r#async = true;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut r#async)
            .add_option(&["--async", "-a"], StoreTrue, "Async")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync");
        ap.refer(&mut separator).add_argument("separator", Store, "Commands separator").required();
        ap.refer(&mut commands).add_argument("arguments", List, "Commands");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        parse_multi_args(&commands, &separator, r#async)
    })
}

pub fn parse_multi_args(xs: &[String], separator: &str, r#async: bool) -> Result<Operation, ParsingError> {
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

    Ok(Operation::Multi(result, r#async))
}

pub fn parse_option_cycle(args: &[String]) -> Result<Operation, ParsingError> {
    let mut option_name = OptionName::default();
    let mut reverse = false;
    let mut candidates = vec![];

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut reverse).add_option(&["--reverse", "-r"], StoreTrue, "Reversed cycle");
        ap.refer(&mut option_name).add_argument("option_name", Store, "Option name").required();
        ap.refer(&mut candidates).add_argument("candidates", Collect, "Candidates").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::UpdateOption(option_name, OptionUpdater::Cycle(reverse, candidates))
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
    } .map(|_| {
        Operation::Page(page)
    })
}

pub fn parse_pdf_index(args: &[String]) -> Result<Operation, ParsingError> {
    use crate::poppler::index::Format;

    let mut r#async = true;
    let mut read_operations = true;
    let mut search_path = false;
    let mut command_line: Vec<String> = vec![];
    let mut fmt = Format::default();
    let mut fmt_separator: Option<String> = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut r#async)
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
    } .map(|_| {
        let command_line = command_line.into_iter().map(Expandable::new).collect();
        Operation::PdfIndex(r#async, read_operations, search_path, command_line, fmt, fmt_separator)
    })
}

pub fn parse_push<T>(args: &[String], op: T) -> Result<Operation, ParsingError>
where T: Fn(String, Option<Meta>, bool, bool) -> Operation {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut paths = Vec::<String>::new();
    let mut force = false;
    let mut show = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show this image after push");
        ap.refer(&mut paths).add_argument("Path", Collect, "Path to resource").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        if paths.len() == 1 {
            let path = paths.remove(0);
            op(path, meta, force, show)
        } else {
            let ops = paths.into_iter().map(|it| op(it, meta.clone(), force, show)).collect();
            Operation::Multi(ops, false)
        }
    })
}

pub fn parse_push_clipboard(args: &[String]) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut as_operation = false;
    let mut selection = ClipboardSelection::default();
    let mut force = false;
    let mut show = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut as_operation)
            .add_option(&["--operation", "-o"], StoreTrue, "As operation")
            .add_option(&["--no-operation", "-O"], StoreTrue, "As path");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut selection)
            .add_option(&["--clipboard", "-c"], StoreConst(ClipboardSelection::Clipboard), "Use `Clipboard`")
            .add_option(&["--primary", "-1", "-p"], StoreConst(ClipboardSelection::Primary), "Use `Primary`")
            .add_option(&["--secondary", "-2", "-s"], StoreConst(ClipboardSelection::Secondary), "Use `Secondary`");
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show this image after push");
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        Operation::PushClipboard(selection, as_operation, meta, force, show)
    })
}

pub fn parse_push_image(args: &[String]) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut paths = Vec::<String>::new();
    let mut expand_level = None;
    let mut force = false;
    let mut show = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut expand_level)
            .add_option(&["--expand", "-e"], StoreConst(Some(0)), "Push and expand")
            .add_option(&["--expand-recursive", "-E"], StoreOption, "Push and expand recursive");
        ap.refer(&mut paths).add_argument("Path", Collect, "Path to image file").required();
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show this image after push");
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        let ops = paths.into_iter().map(|it| Operation::PushImage(Expandable::new(it), meta.clone(), force, show, expand_level)).collect();
        Operation::Multi(ops, false)
    })
}

pub fn parse_push_message(args: &[String]) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut message = s!("");
    let mut show = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show this image after push");
        ap.refer(&mut message).add_argument("Message", Store, "Message").required();
        parse_args(&mut ap, args)
    } .map(|_| Operation::PushMessage(message, new_opt_meta(meta), show))
}

pub fn parse_push_sibling(args: &[String], next: bool) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut force = false;
    let mut show = false;
    let mut clear = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show the found entry");
        ap.refer(&mut clear).add_option(&["--clear", "-c"], StoreTrue, "Clear before push");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::PushSibling(next, clear, new_opt_meta(meta), force, show)
    })
}

pub fn parse_push_url(args: &[String]) -> Result<Operation, ParsingError> {
    let mut meta: Vec<MetaEntry> = vec![];
    let mut urls = Vec::<String>::new();
    let mut force = false;
    let mut show = false;
    let mut entry_type = None;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut meta).add_option(&["--meta", "-m"], Collect, "Meta data");
        ap.refer(&mut force).add_option(&["--force", "-f"], StoreTrue, "Meta data");
        ap.refer(&mut show).add_option(&["--show", "-s"], StoreTrue, "Show the found entry");
        ap.refer(&mut entry_type).add_option(&["--type", "-t", "--as"], StoreOption, "Type (image/archive/pdf)");
        ap.refer(&mut urls).add_argument("URL", Collect, "URL").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        let meta = new_opt_meta(meta);
        let ops = urls.into_iter().map(|it| Operation::PushURL(it, meta.clone(), force, show,entry_type)).collect();
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

pub fn parse_queue(args: &[String]) -> Result<Operation, ParsingError> {
    let mut operation = vec![];
    let mut times = 0;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut times).add_option(&["--times", "-t"], Store, "re-queue times");
        ap.refer(&mut operation).add_argument("Operation", Collect, "Operation").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Queue(operation, times)
    })
}

pub fn parse_record_pre(args: &[String]) -> Result<Operation, ParsingError> {
    let mut operation = vec![];
    let mut minimum_move = 1;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut minimum_move).add_option(&["--minimum", "-m"], Store, "Minium move");
        ap.refer(&mut operation).add_argument("Operation", Collect, "Operation");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::RecordPre(operation, minimum_move)
    })
}

pub fn parse_refresh(args: &[String]) -> Result<Operation, ParsingError> {
    let mut image = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut image).add_option(&["--image", "-i"], StoreTrue, "Refresh image cache");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Refresh(image)
    })
}

pub fn parse_save(args: &[String]) -> Result<Operation, ParsingError> {
    let mut path: String = o!("");
    let mut sources: Vec<Session> = vec![];
    let mut freeze = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut sources).add_option(&["--target", "-t"], Collect, "Target");
        ap.refer(&mut path).add_argument("path", Store, "Save to").required();
        ap.refer(&mut freeze).add_option(&["--freeze", "-F"], StoreTrue, "Insert freezer to stop drawing");
        parse_args(&mut ap, args)
    } .map(|_| {
        if sources.is_empty() {
            sources.push(Session::All);
        }
        Operation::Save(sh::expand_to_pathbuf(&path), sources, freeze)
    })
}

pub fn parse_set_env(args: &[String]) -> Result<Operation, ParsingError> {
    use crate::constant::*;

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
    let mut direction = Direction::Down;
    let mut operation = vec![];
    let mut scroll_size = 1.0;
    let mut crush = false;
    let mut reset_at_end = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut scroll_size).add_option(&["-s", "--size"], Store, "Scroll size (default 1.0) ");
        ap.refer(&mut crush).add_option(&["-c", "--crush"], StoreTrue, "Crush a little space");
        ap.refer(&mut reset_at_end).add_option(&["-r", "--reset"], StoreTrue, "Reset at end");
        ap.refer(&mut direction).add_argument("direction", Store, "left|up|right|down").required();
        ap.refer(&mut operation).add_argument("operation", List, "Operation");
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Scroll(direction, scroll_size, crush, reset_at_end, operation, None)
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
    let mut r#async = true;
    let mut read_as = ReadAs::Ignore;
    let mut search_path = false;
    let mut command_line: Vec<String> = vec![];
    let mut sessions: Vec<Session> = vec![];
    let mut freeze = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut r#async)
            .add_option(&["--async", "-a"], StoreTrue, "Async (Non-blocking)")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync (Blocking)");
        ap.refer(&mut sessions).add_option(&["--session", "-S"], Collect, "Sessions");
        ap.refer(&mut read_as)
            .add_option(&["--binary", "--bin", "-b"], StoreConst(ReadAs::Binary), "As image file")
            .add_option(&["--path", "-P"], StoreConst(ReadAs::Paths), "Read as path list")
            .add_option(&["--operation", "-o"], StoreConst(ReadAs::Operations), "Read operations from stdout");
        ap.refer(&mut search_path).add_option(&["--search-path", "-p"], StoreTrue, SEARCH_PATH_DESC);
        ap.refer(&mut command_line).add_argument("command_line", List, "Command arguments");
        ap.refer(&mut freeze).add_option(&["--freeze", "-F"], StoreTrue, "Insert freezer to stop drawing");
        parse_args(&mut ap, args)
    } .map(|_| {
        let command_line = command_line.into_iter().map(Expandable::new).collect();
        Operation::Shell(r#async, read_as, search_path, command_line, sessions, freeze)
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
    } .map(|_| {
        let command_line = command_line.into_iter().map(Expandable::new).collect();
        Operation::ShellFilter(command_line, search_path)
    })
}

pub fn parse_sort(args: &[String]) -> Result<Operation, ParsingError> {
    let mut fix = false;
    let mut sort_key = SortKey::Natural;
    let mut reverse = false;
    let mut command = vec![];

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
        ap.refer(&mut command).add_argument("command", Collect, "Commadn");
        parse_args(&mut ap, args)
    } .map(|_| {
        let op = if command.is_empty() {
            Operation::Sort(fix, sort_key, reverse)
        } else {
            Operation::Sorter(fix, command, reverse)
        };
        Operation::WithMessage(
            Some(o!("Sorting")),
            Box::new(op))
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
    } .map(|_| op(fix))
}

pub fn parse_timer(args: &[String]) -> Result<Operation, ParsingError> {
    let mut interval_seconds = 1.0;
    let mut name = None;
    let mut op = Vec::<String>::new();
    let mut repeat = Some(1);
    let mut r#async = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut repeat)
            .add_option(&["--repeat", "-r"], StoreOption, "Repeat (0 means infinitely)")
            .add_option(&["--once", "-1"], StoreConst(Some(1)), "Once")
            .add_option(&["--infinity", "-i"], StoreConst(None), "Repeat infinitely");
        ap.refer(&mut name)
            .add_option(&["--name", "-n"], StoreOption, "Name");
        ap.refer(&mut r#async)
            .add_option(&["--async", "-a"], StoreTrue, "Async")
            .add_option(&["--sync", "-s"], StoreFalse, "Sync");
        ap.refer(&mut interval_seconds).add_argument("interval", Store, "Interval").required();
        ap.refer(&mut op).add_argument("operation", Collect, "Operation").required();
        parse_args(&mut ap, args)
    } .map(|_| {
        Operation::Timer(name, op, Duration::from_millis((interval_seconds * 1000.0) as u64), repeat, r#async)
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
        ap.refer(&mut updated.label).add_option(&["--status", "-s"], StoreTrue, "Update status bar");
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
    let mut ignore_views = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut for_rows).add_option(&["--rows", "-r"], StoreTrue, "Set rows");
        ap.refer(&mut cols).add_argument("columns", StoreOption, "Columns");
        ap.refer(&mut rows).add_argument("rows", StoreOption, "Rows");
        ap.refer(&mut ignore_views).add_option(&["--ignore-views", "-i"], StoreTrue, "Ignore the number of views");
        parse_args(&mut ap, args)
    } .and_then(|_| {
        if Some(0) == cols || Some(0) == rows {
            return Err(ParsingError::Fixed("Columns / rows must be greater than 0"))
        }
        Ok(
            if cols.is_some() || rows.is_some() {
                if for_rows {
                    Operation::Views(rows, cols, ignore_views)
                } else {
                    Operation::Views(cols, rows, ignore_views)
                }
            } else {
                Operation::ViewsFellow(for_rows, ignore_views)
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
            "queue" | "que" | "q" =>
                Ok(Session::Queue),
            "paths" | "path" | "P" =>
                Ok(Session::Paths),
            "position" | "pos" | "p" =>
                Ok(Session::Position),
            "mappings" | "mapping" | "map" | "m" =>
                Ok(Session::Mappings),
            "envs" | "env" | "E" =>
                Ok(Session::Envs),
            "filter" | "filters" | "f" =>
                Ok(Session::Filter),
            "reading" | "read" | "r" =>
                Ok(Session::Reading),
            "status" | "stat" | "s" =>
                Ok(Session::Status),
            "markers" | "marker" | "marks" =>
                Ok(Session::Markers),
            "timers" | "timer" | "t" =>
                Ok(Session::Timers),
            "switches" | "switch" | "sw" =>
                Ok(Session::Switches),
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

impl FromStr for Seed {
    type Err = &'static str;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(Seed::new(Some(src)))
    }
}
