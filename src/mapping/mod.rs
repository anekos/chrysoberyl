
use std::fmt;
use std::str::FromStr;

use events::EventName;
use key::Key;
use size::{Region, CoordPx};

pub mod event_mapping;
pub mod input_mapping;
pub mod operation_mapping;
pub mod region_mapping;



#[derive(Debug, Clone, PartialEq)]
pub enum Mapped {
    Operation(String, Vec<String>), // command_name, 
    Event(EventName),
    Input(CoordPx, Key),
    Region(Region, Key, usize), // region, button, cell_index
}

#[derive(Clone, Copy)]
pub enum MappedType {
    Event,
    Input,
}

pub struct Mapping {
    input_history: input_mapping::InputHistory,
    pub operation_mapping: operation_mapping::OperationMapping,
    pub input_mapping: input_mapping::InputMapping,
    pub event_mapping: event_mapping::EventMapping,
    pub region_mapping: region_mapping::RegionMapping,
}


impl Mapping {
    pub fn new() -> Mapping {
        Mapping {
            operation_mapping: operation_mapping::OperationMapping::new(),
            event_mapping: event_mapping::EventMapping::new(),
            input_history: input_mapping::InputHistory::new(),
            input_mapping: input_mapping::InputMapping::new(),
            region_mapping: region_mapping::RegionMapping::new(),
        }
    }

    pub fn register_input(&mut self, key: &[Key], region: Option<Region>, operation: Vec<String>) {
        self.input_mapping.register(key, region, operation);
        self.input_history.clear();
    }

    pub fn register_event(&mut self, event_name: EventName, group: Option<String>, remain: Option<usize>, operation: Vec<String>) {
        self.event_mapping.register(event_name, group, remain, operation);
    }

    pub fn register_operation(&mut self, name: String, operation: Vec<String>) {
        self.operation_mapping.register(name, operation);
    }

    pub fn register_region(&mut self, button: Key, operation: Vec<String>) {
        self.region_mapping.register(button, operation);
    }

    pub fn unregister_input(&mut self, key: &[Key], region: &Option<Region>) {
        self.input_mapping.unregister(key, region);
        self.input_history.clear();
    }

    pub fn unregister_event(&mut self, event_name: &Option<EventName>, group: &Option<String>) {
        self.event_mapping.unregister(event_name, group);
    }

    pub fn unregister_operation(&mut self, name: &str) {
        self.operation_mapping.unregister(name);
    }

    pub fn unregister_region(&mut self, button: &Key) {
        self.region_mapping.unregister(button);
    }

    pub fn matched(&mut self, mapped: &Mapped, width: i32, height: i32, decrease_remain: bool) -> Option<(Vec<Vec<String>>, String)> {
        match *mapped {
            Mapped::Operation(ref name, ref args) =>
                self.operation_mapping.matched(name).map(|mut ops| {
                    ops.extend_from_slice(args);
                    (vec![ops], o!(name))
                }),
            Mapped::Input(coord, ref key) => {
                self.input_history.push(key.clone(), self.input_mapping.depth);
                self.input_mapping.matched(&self.input_history, coord, width, height).map(|(inputs, matched)| {
                    self.input_history.clear();
                    (vec![matched], inputs)
                })
            }
            Mapped::Event(ref event_name) => {
                let ops = self.event_mapping.matched(event_name, decrease_remain);
                if ops.is_empty() {
                    None
                } else {
                    Some((ops, s!(event_name)))
                }
            }
            Mapped::Region(_, ref button, _) =>
                self.region_mapping.matched(button).map(|op| {
                    (vec![op], s!(button))
                })
        }
    }
}


impl Mapped {
    pub fn type_name(&self) -> &str {
        match *self {
            Mapped::Operation(_, _) => "operation",
            Mapped::Input(_, _) => "input",
            Mapped::Event(_) => "event",
            Mapped::Region(_, _, _) => "region",
        }
    }
}

impl fmt::Display for Mapped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Mapped::Operation(ref name, _) => write!(f, "{}", name),
            Mapped::Input(ref coord, ref key) if coord.is_valid() => write!(f, "{} ({})", key, coord),
            Mapped::Input(_, ref key) => write!(f, "{}", key),
            Mapped::Event(ref event_name) => write!(f, "{}", event_name),
            Mapped::Region(ref region, ref button, _) => write!(f, "{} ({})",  button,  region),
        }
    }
}


impl FromStr for MappedType {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        match src {
            "event" | "ev" | "e" =>
                Ok(MappedType::Event),
            "input" | "in" | "i" =>
                Ok(MappedType::Input),
            _ =>
                Err(format!("Invalid type: {}", src))
        }
    }
}


