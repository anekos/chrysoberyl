
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::sync::mpsc::Sender;

use num::Integer;

use option::*;
use operation::Operation;



const NO_VALUE_ERROR: &str = "User switch has no value.";


pub struct UserSwitch {
    app_tx: Sender<Operation>,
    values: VecDeque<Vec<String>>,
}

pub struct DummySwtich {
    name: String
}

pub struct UserSwitchManager {
    app_tx: Sender<Operation>,
    table: HashMap<String, UserSwitch>,
}


impl UserSwitchManager {
    pub fn new(app_tx: Sender<Operation>) -> Self {
        UserSwitchManager {
            app_tx,
            table: HashMap::new()
        }
    }

    pub fn register(&mut self, name: String, values: Vec<Vec<String>>) -> Result<Operation, Box<Error>> {
        let switch = UserSwitch::new(self.app_tx.clone(), values);
        let result = switch.current_operation()?;
        self.table.insert(name, switch);
        Ok(result)
    }

    pub fn get(&mut self, name: &str) -> Option<&mut UserSwitch> {
        self.table.get_mut(name)
    }
}


impl OptionValue for UserSwitch {
    fn toggle(&mut self) -> Result<(), ChryError> {
        self.cycle(false, 1, &[]).and_then(|_| self.send())
    }

    fn cycle(&mut self, reverse: bool, n: usize, _: &[String]) -> Result<(), ChryError> {
        for _ in 0 .. n {
            if reverse {
                let back = self.values.pop_back().expect(NO_VALUE_ERROR);
                self.values.push_front(back);
            } else {
                let front = self.values.pop_front().expect(NO_VALUE_ERROR);
                self.values.push_back(front);
            }
        }
        self.send()
    }
}

impl UserSwitch {
    pub fn new(app_tx: Sender<Operation>, values: Vec<Vec<String>>) -> Self {
        UserSwitch {
            app_tx,
            values: VecDeque::from(values)
        }
    }

    pub fn current(&self) -> Vec<String> {
        self.values.front().cloned().expect(NO_VALUE_ERROR)
    }

    pub fn current_operation(&self) -> Result<Operation, ChryError> {
        Operation::parse_from_vec(&self.current())
    }

    pub fn send(&self) -> Result<(), ChryError> {
        Operation::parse_from_vec(&self.current()).map(|op| {
            self.app_tx.send(op).unwrap()
        })
    }
}


impl DummySwtich {
    pub fn new() -> Self {
        DummySwtich { name: o!("") }
    }

    pub fn rename(&mut self, name: String) {
        self.name = name;
    }
}

impl OptionValue for DummySwtich {
    fn toggle(&mut self) -> Result<(), ChryError> {
        Err(ChryError::InvalidValue(o!(self.name)))
    }

    fn cycle(&mut self, _: bool, n: usize, _: &[String]) -> Result<(), ChryError> {
        if n.is_odd() {
            self.toggle()
        } else {
            Ok(())
        }
    }
}
