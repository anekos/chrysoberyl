
mod parser;



pub struct Candidate {
    key: String,
    tails: Vec<Candidate>,
}
