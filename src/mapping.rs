
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use gdk;

use operation::Operation;



#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Input {
    Key(String),
    MouseButton(NonEntityPosition, u32) // (X, Y), Button
}

#[derive(Clone, Copy)]
pub enum InputType {
    Key,
    MouseButton
}

#[derive(Debug, Clone, Eq)]
pub struct NonEntityPosition {
    pub x: i32,
    pub y: i32,
}


pub struct Mapping {
    table: HashMap<Input, Operation>
}


impl Input {
    pub fn key(key_name: &str) -> Input {
        Input::Key(key_name.to_owned())
    }

    pub fn key_from_event_key(key: &gdk::EventKey) -> Input {
        let keyval = key.as_ref().keyval;
        Input::Key(
            gdk::keyval_name(keyval).unwrap_or_else(|| s!(keyval)))
    }

    pub fn mouse_button(x: i32, y: i32, button: u32) -> Input {
        Input::MouseButton(NonEntityPosition { x: x, y: y }, button)
    }

    pub fn text(&self) -> String {
        match *self {
            Input::Key(ref name) => o!(name),
            Input::MouseButton(ref position, ref button) => format!("{:?}, {}", position.tupled(), button)
        }
    }

    pub fn type_name(&self) -> &str {
        match *self {
            Input::Key(_) => "key",
            Input::MouseButton(_, _) => "mouse_button"
        }
    }
}


impl InputType {
    pub fn input_from_text(&self, text: &str) -> Result<Input, String> {
        match *self {
            InputType::Key =>
                Ok(Input::key(text)),
            InputType::MouseButton => {
                match text.parse() {
                    Ok(button) => Ok(Input::mouse_button(0, 0, button)),
                    Err(err) => Err(s!(err)),
                }
            }
        }
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
        self.table.get(input).cloned()
    }
}


impl NonEntityPosition {
    pub fn tupled(&self) -> (i32, i32) {
        (self.x, self.y)
    }
}

impl PartialEq for NonEntityPosition {
    fn eq(&self, _: &NonEntityPosition) -> bool {
        true
    }
}

impl Hash for NonEntityPosition {
    fn hash<H: Hasher>(&self, state: &mut H) {
        380380.hash(state);
    }
}
