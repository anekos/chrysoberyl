
use std::collections::HashMap;

use gdk;

use operation::Operation;



#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Input {
    Key(String),
    MouseButton(u32)
}

#[derive(Clone, Copy)]
pub enum InputType {
    Key,
    MouseButton
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

    pub fn mouse_button(button: u32) -> Input {
        Input::MouseButton(button)
    }

    pub fn text(&self) -> String {
        match *self {
            Input::Key(ref name) => o!(name),
            Input::MouseButton(button) => s!(button),
        }
    }

    pub fn type_name(&self) -> &str {
        match *self {
            Input::Key(_) => "key",
            Input::MouseButton(_) => "mouse_button"
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
                    Ok(button) => Ok(Input::mouse_button(button)),
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
