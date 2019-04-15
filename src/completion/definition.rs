
use std::borrow::ToOwned;
use std::collections::HashMap;
use std::rc::Rc;

use enum_iterator::IntoEnumIterator;
use pom::parser::*;
use pom::{Parser, TextInput};

use crate::constant::README;
use crate::session::Session;
use crate::util::pom::from_vec_char;



#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Any,
    Directory,
    EventName,
    File,
    Literals(Vec<String>),
    Operator,
    OptionName,
    OptionValue,
    Path,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Argument {
    Flag(Vec<String>, Option<Value>),
    Arg(Value),
    Dots,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Definition {
    pub arguments: HashMap<String, Rc<Vec<Argument>>>,
    pub event_names: Vec<String>,
    pub option_values: HashMap<String, OptionValue>,
    pub options: Vec<String>,
    operations: Vec<String>,
    original_operations: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OptionValue {
    Boolean,
    Enum(Vec<String>),
    StringOrFile,
}


impl Definition {
    pub fn new() -> Self {
        #[derive(Debug)]
        enum In {
            Other,
            Options,
            Events,
            MaskOperators
        }

        let mut original_operations = vec![];
        let mut options: Vec<String> = vec![];
        let mut mask_operators: Vec<String> = vec![];
        let mut option_values = HashMap::<String, OptionValue>::new();
        let mut arguments = HashMap::new();
        let mut event_names = vec![];
        let mut current = In::Other;

        for line in README.lines() {
            match current {
                In::Events => {
                    if line.starts_with('#') {
                        current = In::Other;
                    } else if line.starts_with("| ") {
                        let mut columns = line.split('|').skip(1);
                        match columns.next().unwrap().trim() {
                            "Name" | "----" => (),
                            name => event_names.push(o!(name)),
                        }
                    }
                },
                In::MaskOperators => {
                    if !mask_operators.is_empty() && line.is_empty() {
                        current = In::Other;
                    } else if line.starts_with("- ") {
                        mask_operators.push(o!(line[2..]));
                    }
                },
                In::Options => {
                    if line.starts_with('|') {
                        let mut columns = line.split('|').skip(1);
                        let name = columns.next().unwrap().trim();
                        let tipe = columns.next().unwrap().trim();
                        if name == "Name" || name.starts_with('-') {
                            continue;
                        }
                        match tipe {
                            "string-or-file" => {
                                option_values.insert(o!(name), OptionValue::StringOrFile);
                            }
                            "boolean" => {
                                option_values.insert(o!(name), OptionValue::Boolean);
                            }
                            "mask operators" => {
                                option_values.insert(o!(name), OptionValue::Boolean); // Fake value, replace with real value afterwards.
                            }
                            values => {
                                let values: Vec<String> = values.split('/').map(ToOwned::to_owned).collect();
                                if !values.is_empty() {
                                    option_values.insert(o!(name), OptionValue::Enum(values));
                                }
                            }
                        };
                        options.push(o!(name));
                    } else if line == "## Mask operators" {
                        current = In::MaskOperators;
                    }
                },
                In::Other => {
                    if line.starts_with("## (@") || line.starts_with("## @") {
                        let src = &line[3..];
                        match parse(src, definition) {
                            Err(e) => panic!(format!("Err: {:?} for {:?}", e, line)),
                            Ok((ref ops, ref args)) if ops.is_empty() => panic!(format!("Empty: line={:?}, ops={:?}, args={:?}", line, ops, args)),
                            Ok((ops, args)) => {
                                let args = Rc::new(args);
                                for (i, op) in ops.into_iter().enumerate() {
                                    if arguments.contains_key(&op) {
                                        panic!("Duplicated: {:?}", op);
                                    }
                                    if i == 0 {
                                        original_operations.push(format!("@{}", op.clone()));
                                    }
                                    if !args.is_empty() {
                                        arguments.insert(op, args.clone());
                                    }
                                }
                            }
                        }
                    } else if line == "# Options" {
                        current = In::Options;
                    } else if line == "# Events" {
                        current = In::Events;
                    }
                }
            }
        }

        if let Some(value) = option_values.get_mut("mask-operator") {
            *value = OptionValue::Enum(mask_operators);
        }

        Definition { arguments, event_names, operations: vec![], option_values, options, original_operations }
    }

    pub fn operations(&self) -> Vec<String> {
        self.operations.clone()
    }

    pub fn update_user_operations(&mut self, user_operations: &[String]) {
        self.operations.clear();
        self.operations.extend_from_slice(&self.original_operations);
        self.operations.extend_from_slice(&user_operations);
        self.operations.sort();
    }
}



const SP: &str = " \t()[]<>|";

fn any() -> Parser<char, ()> {
    sym('.').repeat(0..).map(|_| ())
}

fn definition() -> Parser<char, (Vec<String>, Vec<Argument>)> {
    let value = maybe_optional(value);
    let p1 = operations();
    let p2 = dots() | flag() | value.map(Argument::Arg) | literals();

    p1 + (spaces1() * list(p2, spaces1())).opt().map(|it| it.unwrap_or_else(|| vec![]))
}

fn id() -> Parser<char, String> {
    none_of(SP).repeat(1..).map(from_vec_char)
}

fn operation() -> Parser<char, String> {
    (sym('@') * none_of(SP).repeat(1..)).map(from_vec_char)
}

fn operations() -> Parser<char, Vec<String>> {
    let p1 = || list(operation(), sym('|'));
    maybe_grouped(p1)
}

fn literals() -> Parser<char, Argument> {
    let ls = || list(none_of(SP).repeat(1..).map(from_vec_char), sym('|'));
    maybe_grouped(ls).map(|it| Argument::Arg(Value::Literals(it)))
}

fn maybe_grouped<T: 'static, P>(p: P) -> Parser<char, T> where P: Fn() -> Parser<char, T> {
    let pp = sym('(') * p() - sym(')');
    pp | p()
}

fn maybe_optional<T: 'static, P>(p: P) -> Parser<char, T> where P: Fn() -> Parser<char, T> {
    let pp = sym('[') * p() - sym(']');
    (pp | p()) - any()
}

fn flag() -> Parser<char, Argument> {
    let names = || {
        let long = seq("--") * id();
        let short = (sym('-') * none_of(SP)).map(|it| format!("{}", it));
        list(long | short, sym('|'))
    };
    let p = maybe_grouped(names) + (spaces1() * value()).opt();
    (sym('[') * p - sym(']') - any()).map(|(n, v)| Argument::Flag(n, v))
}

fn parse<P, T>(input: &str, p: P) -> Result<T, String> where P: Fn() -> Parser<char, T> {
    let mut input = TextInput::new(input);
    p().parse(&mut input).map_err(|it| s!(it))
}

fn spaces1() -> Parser<char, ()> {
    one_of(" \t").repeat(1..).map(|_| ())
}

fn dots() -> Parser<char, Argument> {
    sym('.').repeat(1..).map(|_| Argument::Dots)
}

fn value() -> Parser<char, Value> {
    (sym('<') * id() - sym('>') - any()).map(|it| {
        match &*it {
            "DIRECTORY" => Value::Directory,
            "EVENT_NAME" => Value::EventName,
            "FILE" => Value::File,
            "OPERATOR" => Value::Operator,
            "OPTION" => Value::OptionName,
            "PATH" => Value::Path,
            "SESSION" => Value::Literals(Session::into_enum_iter().map(|it| s!(it)).collect()),
            "VALUE" => Value::OptionValue,
            _ => Value::Any,
        }
    })
}




#[cfg(test)]#[test]
fn test_parser() {
    use self::Argument::*;
    use self::Value::*;

    assert_eq!(parse("@foo", operations), Ok(vec![o!("foo")]));
    assert_eq!(parse("(@foo|@bar)", operations), Ok(vec![o!("foo"), o!("bar")]));
    assert_eq!(parse("@foo|@bar", operations), Ok(vec![o!("foo"), o!("bar")]));

    assert_eq!(parse("[--cat]", flag), Ok(Flag(vec![o!("cat")], None)));
    assert_eq!(parse("[--cat|-c]", flag), Ok(Flag(vec![o!("cat"), o!("c")], None)));
    assert_eq!(parse("[--neko <MEOW>]", flag), Ok(Flag(vec![o!("neko")], Some(Any))));
    assert_eq!(parse("[--cat|-c <DIRECTORY>]", flag), Ok(Flag(vec![o!("cat"), o!("c")], Some(Directory))));
    assert_eq!(parse("[(--second|-S) <DIRECTORY>]", flag), Ok(Flag(vec![o!("second"), o!("S")], Some(Directory))));
    assert_eq!(parse("[(--operator|-o) <OPERATOR>]", flag), Ok(Flag(vec![o!("operator"), o!("o")], Some(Operator))));

    assert_eq!(
        parse("@cat <FILE>", definition),
        Ok((vec![o!("cat")], vec![Arg(File)])));
    assert_eq!(
        parse("@cat [--meta <KEY_VALUE>] [--force|-f] <PATH>", definition),
        Ok((vec![o!("cat")], vec![Flag(vec![o!("meta")], Some(Any)), Flag(vec![o!("force"), o!("f")], None), Arg(Path)])));

    assert_eq!(
        parse("@cat|@neko", definition),
        Ok((vec![o!("cat"), o!("neko")], vec![])));
    assert_eq!(
        parse("(@cat|@neko) <FILE>", definition),
        Ok((vec![o!("cat"), o!("neko")], vec![Arg(File)])));
    assert_eq!(
        parse("(@cat|@neko) [--long|-s] [(--second|-S) <DIRECTORY>]", definition),
        Ok((
                vec![o!("cat"), o!("neko")],
                vec![Flag(vec![o!("long"), o!("s")], None), Flag(vec![o!("second"), o!("S")], Some(Directory))])));
    assert_eq!(
        parse("(@cat|@neko) [--long|-s] [(--second|-S) <DIRECTORY>] <FILE>", definition),
        Ok((
                vec![o!("cat"), o!("neko")],
                vec![Flag(vec![o!("long"), o!("s")], None), Flag(vec![o!("second"), o!("S")], Some(Directory)), Arg(File)])));
}
