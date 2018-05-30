
use std::collections::HashMap;
use std::rc::Rc;

use pom::parser::*;
use pom::{Parser, TextInput};

use constant::README;
use util::pom::from_vec_char;



#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Any,
    File,
    Directory,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Argument {
    Flag(Vec<String>, Option<Value>),
    Arg(Value),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Definition {
    pub operations: Vec<String>,
    pub arguments: HashMap<String, Rc<Vec<Argument>>>,
}


impl Definition {
    pub fn new() -> Self {
        let mut operations = vec![];
        let mut arguments = HashMap::new();

        for line in README.lines() {
            if line.starts_with("## (@") || line.starts_with("## @") {
                let src = &line[3..];
                match parse(src, definition) {
                    Err(e) => panic!(format!("Err: {:?} for {:?}", e, line)),
                    Ok((ref ops, _)) if ops.is_empty() => panic!(format!("Empty: {:?}", line)),
                    Ok((ops, args)) => {
                        let args = Rc::new(args);
                        for op in ops {
                            if arguments.contains_key(&op) {
                                println!("Duplicated: {:?}", op);
                            } else {
                                operations.push(format!("@{}", op.clone()));
                                if !args.is_empty() {
                                    arguments.insert(op, args.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        Definition { operations, arguments }
    }
}



const SP: &'static str = " \t()[]<>|";


fn definition() -> Parser<char, (Vec<String>, Vec<Argument>)> {
    let p1 = operations();
    let p2 = flag() | value().map(Argument::Arg);

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

fn maybe_grouped<T: 'static, P>(p: P) -> Parser<char, T> where P: Fn() -> Parser<char, T> {
    let pp = sym('(') * p() - sym(')');
    pp | p()
}

fn flag() -> Parser<char, Argument> {
    let names = || {
        let long = seq("--") * id();
        let short = (sym('-') * none_of(SP)).map(|it| format!("{}", it));
        list(long | short, sym('|'))
    };
    (sym('[') * maybe_grouped(names) + (spaces1() * value()).opt() - sym(']')).map(|(n, v)| Argument::Flag(n, v))
}

fn parse<P, T>(input: &str, p: P) -> Result<T, String> where P: Fn() -> Parser<char, T> {
    let mut input = TextInput::new(input);
    p().parse(&mut input).map_err(|it| s!(it))
}

fn spaces1() -> Parser<char, ()> {
    one_of(" \t").repeat(1..).map(|_| ())
}

fn value() -> Parser<char, Value> {
    (sym('<') * id() - sym('>')).map(|it| {
        match &*it {
            "FILE" => Value::File,
            "DIRECTORY" => Value::Directory,
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
    assert_eq!(parse("[--neko <VALUE>]", flag), Ok(Flag(vec![o!("neko")], Some(Any))));
    assert_eq!(parse("[--cat|-c <DIRECTORY>]", flag), Ok(Flag(vec![o!("cat"), o!("c")], Some(Directory))));
    assert_eq!(parse("[(--second|-S) <DIRECTORY>]", flag), Ok(Flag(vec![o!("second"), o!("S")], Some(Directory))));

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
