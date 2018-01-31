
use std::error::Error;

use pom::parser::*;
use pom::{Parser, TextInput};

use gui::completion::Candidate;

use readme;



// pub fn parse(source: &str) -> Candidate {
//     let mut tails = vec![];
//
//     for line in readme::body() {
//     }
//
//     Candidate { key: o!(""), tails }
// }
//
//
// // fn part() -> Parser<char, Vec<Candidate>> {
// // }
//
// fn group() -> Parser<char, Vec<Candidate>> {
//     sym('(') * list(call(group), sym('|')) - sym(')')
// }
//
// fn operation() -> Parser<char, Vec<Candidate>> {
//     group()
// }
//
//
// fn parse_line(line: &str) -> Result<Candidate, Box<Error>> {
//     Ok(Candidate { key: o!(""), tails: vec![]})
// }
