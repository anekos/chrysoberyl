
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::Sender;

use option::*;
use operation::Operation;


const NO_VALUE_ERROR: &'static str = "User switch has no value.";


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
            app_tx: app_tx,
            table: HashMap::new()
        }
    }

    pub fn register(&mut self, name: String, values: Vec<Vec<String>>) {
        self.table.insert(name, UserSwitch::new(self.app_tx.clone(), values));
    }

    pub fn get(&mut self, name: &str) -> Option<&mut UserSwitch> {
        self.table.get_mut(name)
    }
}


impl OptionValue for UserSwitch {
    fn toggle(&mut self) -> Result {
        self.cycle(false).and_then(|_| self.send())
    }

    fn cycle(&mut self, reverse: bool) -> Result {
        if reverse {
            let front = self.values.pop_front().expect(NO_VALUE_ERROR);
            self.values.push_back(front);
        } else {
            let back = self.values.pop_back().expect(NO_VALUE_ERROR);
            self.values.push_front(back);
        }
        self.send()
    }
}

impl UserSwitch {
    pub fn new(app_tx: Sender<Operation>, values: Vec<Vec<String>>) -> Self {
        UserSwitch {
            app_tx: app_tx,
            values: VecDeque::from(values)
        }
    }

    pub fn current(&self) -> Vec<String> {
        self.values.front().cloned().expect(NO_VALUE_ERROR)
    }

    pub fn send(&self) -> Result {
        Ok(())
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
    fn toggle(&mut self) -> Result {
        Err(format!("Invalid option name: {}", self.name))
    }

    fn cycle(&mut self, _: bool) -> Result {
        self.toggle()
    }
}
