
use std::collections::HashMap;



pub struct EventMapping {
    pub table: HashMap<String, EventMappingEntry>,
}

pub struct EventMappingEntry {
    pub table: HashMap<Option<String>, Vec<Vec<String>>>
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventKey {
    name: String,
    group: Option<String>,
}



impl EventMapping {
    pub fn new() -> Self {
        EventMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, event_name: String, group: Option<String>, operation: Vec<String>) {
        if let Some(entry) = self.table.get_mut(&event_name) {
            entry.register(group, operation);
            return;
        }

        let mut entry = EventMappingEntry::new();
        entry.register(group, operation);
        self.table.insert(event_name, entry);
    }

    pub fn matched(&self, event_name: &str) -> Vec<Vec<String>> {
        self.table.get(event_name).map(|it| it.entries()).unwrap_or_else(|| vec![])
    }
}

impl EventMappingEntry {
    pub fn new() -> Self {
        EventMappingEntry { table: HashMap::new() }
    }

    pub fn register(&mut self, group: Option<String>, operation: Vec<String>) {
        if let Some(entry) = self.table.get_mut(&group) {
            entry.push(operation);
            return;
        }

        self.table.insert(group, vec![operation]);
    }

    pub fn entries(&self) -> Vec<Vec<String>> {
        let mut result = vec![];
        for ops in self.table.values() {
            result.extend_from_slice(ops)
        }
        result
    }
}
