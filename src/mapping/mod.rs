
use gdk;

use operation::Operation;


pub mod key_mapping;
pub mod mouse_mapping;


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Input {
    Key(String),
    MouseButton((i32, i32), u32) // (X, Y), Button
}

#[derive(Clone, Copy)]
pub enum InputType {
    Key,
    MouseButton
}


pub struct Mapping {
    key_mapping: key_mapping::KeyMapping,
    mouse_mapping: mouse_mapping::MouseMapping,
}


impl Mapping {
    pub fn new() -> Mapping {
        Mapping {
            key_mapping: key_mapping::KeyMapping::new(),
            mouse_mapping: mouse_mapping::MouseMapping::new(),
        }
    }

    pub fn register_key(&mut self, key: &str, operation: &[String]) {
        self.key_mapping.register(key.to_owned(), operation);
    }

    pub fn register_mouse(&mut self, button: u32, area: Option<mouse_mapping::Area>, operation: &[String]) {
        self.mouse_mapping.register(button, area, operation);
    }

    pub fn matched(&self, input: &Input, width: i32, height: i32) -> Option<Result<Operation, String>> {
        match *input {
            Input::Key(ref key) =>
                self.key_mapping.matched(key),
            Input::MouseButton((x, y), ref button) =>
                self.mouse_mapping.matched(*button, x, y, width, height),
        }
    }
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
        Input::MouseButton((x, y), button)
    }

    pub fn text(&self) -> String {
        match *self {
            Input::Key(ref name) => o!(name),
            Input::MouseButton(ref position, ref button) => format!("{:?}, {}", position, button)
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
