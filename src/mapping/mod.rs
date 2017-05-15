
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
    key_input_history: key_mapping::KeyInputHistory,
    key_mapping: key_mapping::KeyMapping,
    mouse_mapping: mouse_mapping::MouseMapping,
}


impl Mapping {
    pub fn new() -> Mapping {
        Mapping {
            key_input_history: key_mapping::KeyInputHistory::new(),
            key_mapping: key_mapping::KeyMapping::new(),
            mouse_mapping: mouse_mapping::MouseMapping::new(),
        }
    }

    pub fn register_key(&mut self, key: Vec<String>, operation: Vec<String>) {
        self.key_mapping.register(key, operation);
    }

    pub fn register_mouse(&mut self, button: u32, area: Option<mouse_mapping::Area>, operation: &[String]) {
        self.mouse_mapping.register(button, area, operation);
    }

    pub fn matched(&mut self, input: &Input, width: i32, height: i32) -> Option<Result<Operation, String>> {
        match *input {
            Input::Key(ref key) => {
                self.key_input_history.push(key.clone(), self.key_mapping.depth);
                self.key_mapping.matched(&self.key_input_history)
            }
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
        use gdk;

        let keyval = key.as_ref().keyval;
        let state = key.get_state();

        let mut name = o!("");

        if state.contains(gdk::CONTROL_MASK) { name.push_str("C-"); }
        if state.contains(gdk::HYPER_MASK) { name.push_str("H-"); }
        if state.contains(gdk::META_MASK) { name.push_str("M-"); }
        if state.contains(gdk::MOD1_MASK) { name.push_str("A-"); }
        if state.contains(gdk::SUPER_MASK) { name.push_str("S-"); }

        name.push_str(&gdk::keyval_name(keyval).unwrap_or_else(|| s!(keyval)));

        Input::Key(name)
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
