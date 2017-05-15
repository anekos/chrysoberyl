
use std::collections::{HashMap, VecDeque};

use operation::Operation;



type KeySequence = Vec<String>;
type OperationCode = Vec<String>;


pub struct KeyMapping {
    pub depth: usize,
    table: HashMap<String, MappingEntry>
}

pub struct KeyInputHistory {
    pub entries: VecDeque<String>
}

pub enum MappingEntry {
    Code(Vec<String>),
    Sub(Box<KeyMapping>)
}


impl KeyMapping {
    pub fn new() -> KeyMapping {
        KeyMapping { depth: 1, table: HashMap::new() }
    }

    pub fn new_entry(keys: KeySequence, operation: OperationCode) -> MappingEntry {
        if !keys.is_empty() {
            let mut result = KeyMapping { depth: 1, table: HashMap::new() };
            result.register(keys, operation);
            MappingEntry::Sub(Box::new(result))
        } else {
            MappingEntry::Code(operation)
        }
    }

    pub fn register(&mut self, keys: KeySequence, operation: OperationCode) {
        self._register(keys, operation);
        self.update_depth();
    }

    fn _register(&mut self, keys: KeySequence, operation: OperationCode) {
        use self::MappingEntry::*;

        if let Some((head, tail)) = keys.split_first() {
            let tail = tail.to_vec();
            if let Some(ref mut entry) = self.table.get_mut(head) {
                match **entry {
                    Code(_) =>
                        **entry = KeyMapping::new_entry(tail, operation),
                    Sub(_) if tail.is_empty() =>
                        **entry = KeyMapping::new_entry(tail, operation),
                    Sub(ref mut sub) =>
                        sub.register(tail, operation)
                }
                return
            }
            self.table.insert(head.clone(), KeyMapping::new_entry(tail, operation));
        } else {
            panic!("Empty key sequence");
        }
    }

    pub fn matched(&self, history: &KeyInputHistory) -> Option<Result<Operation, String>> {
        let entries = &history.entries;
        let len = entries.len();
        for i in 0..len {
            let mut mapping = self;
            for (j, entry) in entries.iter().enumerate().take(len).skip(i) {
                if let Some(entry) = mapping.table.get(entry) {
                    match *entry {
                        MappingEntry::Sub(ref sub) =>
                            mapping = sub,
                        MappingEntry::Code(ref code) if j == len - 1 =>
                            return Some(Operation::parse_from_vec(code)),
                        _ =>
                            ()
                    }
                }
            }
        }
        None
    }

    pub fn update_depth(&mut self) {
        let sub_max = self.table.iter().map(|(_, entry)| {
            match *entry {
                MappingEntry::Sub(ref sub) => sub.depth,
                _ => 0
            }
        }).max();

        self.depth = sub_max.unwrap_or(0) + 1;
    }
}


impl KeyInputHistory {
    pub fn new() -> KeyInputHistory {
        KeyInputHistory { entries: VecDeque::new() }
    }

    pub fn push(&mut self, key: String, depth: usize) {
        self.entries.push_back(key);
        while depth < self.entries.len() {
            self.entries.pop_front();
        }
    }
}
