
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::Sender;

use num::Integer;

use operation::Operation;
use option::*;
use util::num::cycle_n;




pub struct UserSwitch {
    app_tx: Sender<Operation>,
    value: usize,
    values: Vec<Vec<String>>,
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
        self.cycle(false, 1, &[])
    }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse()
            .map_err(|it| ChryError::Standard(format!("Invalid value: {} ({})", value, it)))
            .and_then(|value| {
                if value < self.values.len() {
                    if self.value != value {
                        self.value = value;
                        return self.send()
                    }
                    Ok(())
                } else {
                    Err(ChryError::Fixed("Too large"))
                }
            })
    }

    fn cycle(&mut self, reverse: bool, n: usize, _: &[String]) -> Result<(), ChryError> {
        let new_value = cycle_n(self.value, self.values.len(), reverse, n);
        if new_value != self.value {
            self.value = new_value;
            return self.send()
        }
        Ok(())
    }
}

impl UserSwitch {
    pub fn new(app_tx: Sender<Operation>, values: Vec<Vec<String>>) -> Self {
        UserSwitch {
            app_tx,
            value: 0,
            values,
        }
    }

    pub fn current(&self) -> Vec<String> {
        self.values[self.value].clone()
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
