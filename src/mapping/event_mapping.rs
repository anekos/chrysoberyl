
use std::collections::HashMap;

use crate::events::EventName;



pub struct EventMapping {
    pub table: HashMap<EventName, Vec<EventMappingEntry>>,
}

pub struct EventMappingEntry {
    pub group: Option<String>,
    pub operation: Vec<String>,
    pub remain: Option<usize>,
}



impl EventMapping {
    pub fn new() -> Self {
        EventMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, event_name: EventName, group: Option<String>, remain: Option<usize>, operation: Vec<String>) {
        let entry = EventMappingEntry { group, operation, remain };

        if let Some(entries) = self.table.get_mut(&event_name) {
            return entries.push(entry);
        }

        let entries = vec![entry];
        self.table.insert(event_name, entries);
    }

    pub fn unregister(&mut self, event_name: &Option<EventName>, group: &Option<String>) {
        match *event_name {
            Some(ref event_name) => {
                if_let_some!(entries = self.table.get_mut(event_name));
                entries.retain(|it| it.group != *group);
            },
            None => {
                for entries in self.table.values_mut() {
                    entries.retain(|it| it.group != *group)
                }
            }
        }

        self.table.retain(|_, it| !it.is_empty());
    }

    pub fn matched(&mut self, event_name: &EventName, decrease_remain: bool) -> Vec<Vec<String>> {
        if_let_some!(entries = self.table.get_mut(event_name), vec![]);
        let result = entries.iter().map(|it| it.operation.clone()).collect();

        if decrease_remain{
            entries.retain(|it| it.remain.map(|it| 1 < it).unwrap_or(true));
            for it in entries.iter_mut() {
                if let Some(it) = it.remain.as_mut() {
                    *it += 1;
                }
            }
        }

        result
    }
}
