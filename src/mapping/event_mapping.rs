
use std::collections::HashMap;

use events::EventName;



pub struct EventMapping {
    pub table: HashMap<EventName, Vec<EventMappingEntry>>,
}

pub struct EventMappingEntry {
    group: Option<String>,
    operation: Vec<String>,
}



impl EventMapping {
    pub fn new() -> Self {
        EventMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, event_name: EventName, group: Option<String>, operation: Vec<String>) {
        let entry = EventMappingEntry { group: group, operation: operation };

        if let Some(entries) = self.table.get_mut(&event_name) {
            return entries.push(entry);
        }

        let entries = vec![entry];
        self.table.insert(event_name, entries);
    }

    pub fn unregister(&mut self, event_name: &Option<EventName>, group: &Option<String>) {
        match *event_name {
            Some(ref event_name) => {
                if_let_some!(entries = self.table.get_mut(event_name), ());
                entries.retain(|it| it.group == *group)
            },
            None => {
                for entries in self.table.values_mut() {
                    entries.retain(|it| it.group == *group)
                }
            }
        }
    }

    pub fn matched(&self, event_name: &EventName) -> Vec<Vec<String>> {
        if_let_some!(entries = self.table.get(event_name), vec![]);
        entries.iter().map(|it| it.operation.clone()).collect()
    }
}
