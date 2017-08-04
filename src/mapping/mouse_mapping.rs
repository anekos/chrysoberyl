
use std::collections::HashMap;

use size::Region;



pub struct MouseMapping {
    pub table: HashMap<u32, Vec<WithRegion>>
}

pub struct WithRegion {
    pub operation: Vec<String>,
    pub region: Option<Region>
}


impl MouseMapping {
    pub fn new() -> MouseMapping {
        MouseMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, button: u32, region: Option<Region>, operation: Vec<String>) {
        let entry = WithRegion { operation: operation.to_vec(), region: region };
        if region.is_some() {
            if let Some(mut entries) = self.table.get_mut(&button) {
                entries.retain(|it| it.region != region);
                entries.push(entry);
                return;
            }
        }
        self.table.insert(button, vec![entry]);
    }

    pub fn unregister(&mut self, button: &u32, region: &Option<Region>) {
        let is_empty = {
            if_let_some!(entries = self.table.get_mut(button), ());
            entries.retain(|it| it.region != *region);
            entries.is_empty()
        };
        if is_empty {
            self.table.remove(button);
        }
    }

    pub fn matched(&self, button: u32, x: i32, y: i32, width: i32, height: i32) -> Option<Vec<String>> {
        self.table.get(&button).and_then(|entries| {
            let mut found = None;

            for entry in entries.iter() {
                if let Some(area) = entry.region {
                    if area.contains(x, y, width, height) {
                        found = Some(entry.operation.clone());
                        break;
                    }
                } else if found.is_none() {
                    found = Some(entry.operation.clone());
                }
            }

            found
        })
    }
}
