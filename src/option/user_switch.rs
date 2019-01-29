
use std::cmp::{Ord, Ordering};
use std::collections::{hash_map, HashMap};
use std::slice;
use std::sync::mpsc::Sender;

use num::Integer;

use crate::errors::{AppResult, AppResultU, Error as AppError, ErrorKind};
use crate::operation::Operation;
use crate::option::*;
use crate::util::num::cycle_n;




pub struct UserSwitch {
    app_tx: Sender<Operation>,
    serial: usize,
    value: usize,
    values: Vec<Vec<String>>,
}

pub struct DummySwtich {
    name: String
}

pub struct UserSwitchManager {
    app_tx: Sender<Operation>,
    serial: usize,
    table: HashMap<String, UserSwitch>,
}


const OVERFLOW: ErrorKind = ErrorKind::Fixed("Overflow");


impl UserSwitchManager {
    pub fn new(app_tx: Sender<Operation>) -> Self {
        UserSwitchManager {
            app_tx,
            serial: 0,
            table: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, values: Vec<Vec<String>>) -> AppResult<Operation> {
        let switch = UserSwitch::new(self.app_tx.clone(), self.serial, values);
        let result = switch.current_operation()?;
        self.table.insert(name, switch);
        self.serial += 1;
        Ok(result)
    }

    pub fn get(&mut self, name: &str) -> Option<&mut UserSwitch> {
        self.table.get_mut(name)
    }

    pub fn iter(&self) -> hash_map::Iter<String, UserSwitch> {
        self.table.iter()
    }
}


impl Ord for UserSwitch {
    fn cmp(&self, other: &UserSwitch) -> Ordering {
        self.serial.cmp(&other.serial)
    }
}

impl PartialOrd for UserSwitch {
    fn partial_cmp(&self, other: &UserSwitch) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for UserSwitch {}

impl PartialEq for UserSwitch {
    fn eq(&self, other: &UserSwitch) -> bool {
        self.serial == other.serial
    }
}

impl OptionValue for UserSwitch {
    fn cycle(&mut self, reverse: bool, n: usize, _: &[String]) -> AppResultU {
        let new_value = cycle_n(self.value, self.values.len(), reverse, n);
        if new_value != self.value {
            self.value = new_value;
            return self.send()
        }
        Ok(())
    }

    fn decrement(&mut self, delta: usize) -> AppResultU {
        if_let_some!(new_value = self.value.checked_sub(delta), Err(OVERFLOW)?);
        self.value = new_value;
        self.send()
    }

    fn disable(&mut self) -> AppResultU {
        self.unset()
    }

    fn enable(&mut self) -> AppResultU {
        self.set("1")
    }

    fn increment(&mut self, delta: usize) -> AppResultU {
        if_let_some!(new_value = self.value.checked_add(delta), Err(OVERFLOW)?);
        if new_value < self.values.len() {
            self.value = new_value;
            self.send()
        } else {
            Err(OVERFLOW)?
        }
    }

    fn is_enabled(&self) -> AppResult<bool> {
        Ok(self.value == 1)
    }

    fn set(&mut self, value: &str) -> AppResultU {
        value.parse()
            .map_err(|it| AppError::from(ErrorKind::Standard(format!("Invalid value: {} ({})", value, it))))
            .and_then(|value: usize| {
                if value == 0 {
                    Err(ErrorKind::Fixed("Zero is invalid"))?
                } else if value <= self.values.len() {
                    if self.value != value {
                        self.value = value - 1;
                        return self.send()
                    }
                    Ok(())
                } else {
                    Err(ErrorKind::Fixed("Too large"))?
                }
            })?;
        Ok(())
    }

    fn set_from_count(&mut self, count: Option<usize>) -> AppResultU {
        if let Some(count) = count {
            self.set(&format!("{}", count - 1))
        } else {
            self.unset()
        }
    }

    fn toggle(&mut self) -> AppResultU {
        self.cycle(false, 1, &[])
    }

    fn unset(&mut self) -> AppResultU {
        self.value = 0;
        self.send()
    }
}

impl UserSwitch {
    pub fn new(app_tx: Sender<Operation>, serial: usize, values: Vec<Vec<String>>) -> Self {
        UserSwitch {
            app_tx,
            serial,
            value: 0,
            values,
        }
    }

    pub fn current(&self) -> Vec<String> {
        self.values[self.value].clone()
    }

    pub fn current_operation(&self) -> AppResult<Operation> {
        Operation::parse_from_vec(&self.current())
    }

    pub fn current_value(&self) -> usize {
        self.value + 1
    }

    pub fn iter(&self) -> slice::Iter<Vec<String>> {
        self.values.iter()
    }

    pub fn send(&self) -> AppResultU {
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
    fn toggle(&mut self) -> AppResultU {
        Err(ErrorKind::InvalidValue(o!(self.name)))?
    }

    fn cycle(&mut self, _: bool, n: usize, _: &[String]) -> AppResultU {
        if n.is_odd() {
            self.toggle()
        } else {
            Ok(())
        }
    }
}
