
use pom::parser::*;
use pom::{Parser, TextInput};

use constant::README;
use util::pom::from_vec_char;



#[derive(Debug, PartialEq, Eq)]
enum Value {
    Any,
    File,
    Directory,
}

#[derive(Debug, PartialEq, Eq)]
enum Parsed {
    PFlag(Vec<String>, Option<Value>),
    PArg(Value),
}


pub struct Definition {
    pub operations: Vec<String>,
}


impl Definition {
    pub fn new() -> Self {
        let mut operations = vec![];

        for line in README.lines() {
            if line.starts_with("## (@") || line.starts_with("## @") {
                let src = &line[3..];
                match parse(src, definition) {
                    Err(e) => panic!(format!("Err: {:?} for {:?}", e, line)),
                    Ok((ref ops, _)) if ops.is_empty() => panic!(format!("Empty: {:?}", line)),
                    Ok((ops, _args)) => {
                        for op in ops {
                            operations.push(format!("@{}", op));
                        }
                    }
                }
            }
        }

        Definition { operations }
    }
}



const SP: &'static str = " \t()[]<>|";


fn definition() -> Parser<char, (Vec<String>, Vec<Parsed>)> {
    let p1 = operations();
    let p2 = flag() | value().map(Parsed::PArg);

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

fn flag() -> Parser<char, Parsed> {
    let names = || {
        let long = seq("--") * id();
        let short = (sym('-') * none_of(SP)).map(|it| format!("{}", it));
        list(long | short, sym('|'))
    };
    (sym('[') * maybe_grouped(names) + (spaces1() * value()).opt() - sym(']')).map(|(n, v)| Parsed::PFlag(n, v))
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
    use self::Parsed::*;
    use self::Value::*;

    assert_eq!(parse("@foo", operations), Ok(vec![o!("foo")]));
    assert_eq!(parse("(@foo|@bar)", operations), Ok(vec![o!("foo"), o!("bar")]));
    assert_eq!(parse("@foo|@bar", operations), Ok(vec![o!("foo"), o!("bar")]));

    assert_eq!(parse("[--cat]", flag), Ok(PFlag(vec![o!("cat")], None)));
    assert_eq!(parse("[--cat|-c]", flag), Ok(PFlag(vec![o!("cat"), o!("c")], None)));
    assert_eq!(parse("[--neko <VALUE>]", flag), Ok(PFlag(vec![o!("neko")], Some(Any))));
    assert_eq!(parse("[--cat|-c <DIRECTORY>]", flag), Ok(PFlag(vec![o!("cat"), o!("c")], Some(Directory))));
    assert_eq!(parse("[(--second|-S) <DIRECTORY>]", flag), Ok(PFlag(vec![o!("second"), o!("S")], Some(Directory))));

    assert_eq!(
        parse("@cat|@neko", definition),
        Ok((vec![o!("cat"), o!("neko")], vec![])));
    assert_eq!(
        parse("(@cat|@neko) <FILE>", definition),
        Ok((vec![o!("cat"), o!("neko")], vec![PArg(File)])));
    assert_eq!(
        parse("(@cat|@neko) [--long|-s] [(--second|-S) <DIRECTORY>]", definition),
        Ok((
                vec![o!("cat"), o!("neko")],
                vec![PFlag(vec![o!("long"), o!("s")], None), PFlag(vec![o!("second"), o!("S")], Some(Directory))])));
    assert_eq!(
        parse("(@cat|@neko) [--long|-s] [(--second|-S) <DIRECTORY>] <FILE>", definition),
        Ok((
                vec![o!("cat"), o!("neko")],
                vec![PFlag(vec![o!("long"), o!("s")], None), PFlag(vec![o!("second"), o!("S")], Some(Directory)), PArg(File)])));
}
