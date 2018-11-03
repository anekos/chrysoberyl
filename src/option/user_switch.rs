
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


const OVERFLOW: Result<(), ChryError> = Err(ChryError::Fixed("Overflow"));


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
    fn cycle(&mut self, reverse: bool, n: usize, _: &[String]) -> Result<(), ChryError> {
        let new_value = cycle_n(self.value, self.values.len(), reverse, n);
        if new_value != self.value {
            self.value = new_value;
            return self.send()
        }
        Ok(())
    }

    fn decrement(&mut self, delta: usize) -> Result<(), ChryError> {
        if_let_some!(new_value = self.value.checked_sub(delta), OVERFLOW);
        self.value = new_value;
        self.send()
    }

    fn disable(&mut self) -> Result<(), ChryError> {
        self.unset()
    }

    fn enable(&mut self) -> Result<(), ChryError> {
        self.set("1")
    }

    fn increment(&mut self, delta: usize) -> Result<(), ChryError> {
        if_let_some!(new_value = self.value.checked_add(delta), OVERFLOW);
        if new_value < self.values.len() {
            self.value = new_value;
            self.send()
        } else {
            OVERFLOW
        }
    }

    fn is_enabled(&self) -> Result<bool, ChryError> {
        Ok(self.value == 1)
    }

    fn set(&mut self, value: &str) -> Result<(), ChryError> {
        value.parse()
            .map_err(|it| ChryError::Standard(format!("Invalid value: {} ({})", value, it)))
            .and_then(|value: usize| {
                if value == 0 {
                    Err(ChryError::Fixed("Zero is invalid"))
                } else if value <= self.values.len() {
                    if self.value != value {
                        self.value = value - 1;
                        return self.send()
                    }
                    Ok(())
                } else {
                    Err(ChryError::Fixed("Too large"))
                }
            })
    }

    fn set_from_count(&mut self, count: Option<usize>) -> Result<(), ChryError> {
        if let Some(count) = count {
            self.set(&format!("{}", count - 1))
        } else {
            self.unset()
        }
    }

    fn toggle(&mut self) -> Result<(), ChryError> {
        self.cycle(false, 1, &[])
    }

    fn unset(&mut self) -> Result<(), ChryError> {
        self.value = 0;
        self.send()
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
