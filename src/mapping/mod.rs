
use std::fmt;

use events::EventName;
use key::Key;
use size::{Region, CoordPx};

pub mod event_mapping;
pub mod region_mapping;
pub mod unified_mapping;



#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Unified(CoordPx, Key),
    Event(EventName),
    Region(Region, Key, usize), // region, button, cell_index
}

#[derive(Clone, Copy)]
pub enum InputType {
    Unified,
    Event,
}

pub struct Mapping {
    input_history: unified_mapping::InputHistory,
    pub unified_mapping: unified_mapping::UnifiedMapping,
    pub event_mapping: event_mapping::EventMapping,
    pub region_mapping: region_mapping::RegionMapping,
}


impl Mapping {
    pub fn new() -> Mapping {
        Mapping {
            input_history: unified_mapping::InputHistory::new(),
            unified_mapping: unified_mapping::UnifiedMapping::new(),
            event_mapping: event_mapping::EventMapping::new(),
            region_mapping: region_mapping::RegionMapping::new(),
        }
    }

    pub fn register_unified(&mut self, key: &[Key], region: Option<Region>, operation: Vec<String>) {
        self.unified_mapping.register(key, region, operation);
        self.input_history.clear();
    }

    pub fn register_event(&mut self, event_name: EventName, group: Option<String>, remain: Option<usize>, operation: Vec<String>) {
        self.event_mapping.register(event_name, group, remain, operation);
    }

    pub fn register_region(&mut self, button: Key, operation: Vec<String>) {
        self.region_mapping.register(button, operation);
    }

    pub fn unregister_unified(&mut self, key: &[Key], region: &Option<Region>) {
        self.unified_mapping.unregister(key, region);
        self.input_history.clear();
    }

    pub fn unregister_event(&mut self, event_name: &Option<EventName>, group: &Option<String>) {
        self.event_mapping.unregister(event_name, group);
    }

    pub fn unregister_region(&mut self, button: &Key) {
        self.region_mapping.unregister(button);
    }

    pub fn matched(&mut self, input: &Input, width: i32, height: i32, decrease_remain: bool) -> Option<(Vec<Vec<String>>, String)> {
        match *input {
            Input::Unified(coord, ref key) => {
                self.input_history.push(key.clone(), self.unified_mapping.depth);
                self.unified_mapping.matched(&self.input_history, coord, width, height).map(|(inputs, matched)| {
                    self.input_history.clear();
                    (vec![matched], inputs)
                })
            }
            Input::Event(ref event_name) => {
                let ops = self.event_mapping.matched(event_name, decrease_remain);
                if ops.is_empty() {
                    None
                } else {
                    Some((ops, s!(event_name)))
                }
            }
            Input::Region(_, ref button, _) =>
                self.region_mapping.matched(button).map(|op| {
                    (vec![op], s!(button))
                })
        }
    }
}


impl Input {
    pub fn type_name(&self) -> &str {
        match *self {
            Input::Unified(_, _) => "unified",
            Input::Event(_) => "event",
            Input::Region(_, _, _) => "region",
        }
    }
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Input::Unified(ref coord, ref key) if coord.is_valid() => write!(f, "{} ({})", key, coord),
            Input::Unified(_, ref key) => write!(f, "{}", key),
            Input::Event(ref event_name) => write!(f, "{}", event_name),
            Input::Region(ref region, ref button, _) => write!(f, "{} ({})",  button,  region),
        }
    }
}
