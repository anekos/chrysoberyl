
use std::collections::HashMap;

use operation::Operation;



#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Input {
    Key(String),
    MouseButton(u32)
}


pub struct Mapping {
    table: HashMap<Input, Operation>
}


impl Input {
    pub fn key(key_name: &str) -> Input {
        Input::Key(normalize_key_name(key_name))
    }

    pub fn mouse_button(button: u32) -> Input {
        Input::MouseButton(button)
    }
}


impl Mapping {
    pub fn new() -> Mapping {
        Mapping { table: HashMap::new() }
    }

    pub fn register(&mut self, input: Input, operation: Operation) {
        self.table.insert(input, operation);
    }

    pub fn matched(&self, input: &Input) -> Option<Operation> {
        self.table.get(input).map(|it| it.clone())
    }
}


fn normalize_key_name(key_name: &str) -> String {
    key_name.to_lowercase()
}
