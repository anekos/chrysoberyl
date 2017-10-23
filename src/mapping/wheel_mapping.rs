
use std::collections::HashMap;

use gtk_wrapper::ScrollDirection;



pub struct WheelMapping {
    pub table: HashMap<ScrollDirection, Vec<String>>
}




impl WheelMapping {
    pub fn new() -> Self {
        WheelMapping { table: HashMap::new() }
    }

    pub fn register(&mut self, direction: ScrollDirection, operation: Vec<String>) {
        self.table.insert(direction, operation);
    }

    pub fn unregister(&mut self, direction: ScrollDirection) {
        self.table.remove(&direction);
    }

    pub fn matched(&self, direction: ScrollDirection) -> Option<Vec<String>> {
        self.table.get(&direction).cloned()
    }
}
