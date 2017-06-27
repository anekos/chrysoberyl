
use gdk;


pub mod event_mapping;
pub mod key_mapping;
pub mod mouse_mapping;
pub mod region_mapping;

use size::Region;



#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Key(String),
    MouseButton((i32, i32), u32), // (X, Y), Button
    Event(String), // event name
    Region(Region, u32, usize), // region, button, cell_index
}

#[derive(Clone, Copy)]
pub enum InputType {
    Key,
    MouseButton,
    Event,
}


pub struct Mapping {
    key_input_history: key_mapping::KeyInputHistory,
    pub key_mapping: key_mapping::KeyMapping,
    pub mouse_mapping: mouse_mapping::MouseMapping,
    pub event_mapping: event_mapping::EventMapping,
    pub region_mapping: region_mapping::RegionMapping,
}


impl Mapping {
    pub fn new() -> Mapping {
        Mapping {
            key_input_history: key_mapping::KeyInputHistory::new(),
            key_mapping: key_mapping::KeyMapping::new(),
            mouse_mapping: mouse_mapping::MouseMapping::new(),
            event_mapping: event_mapping::EventMapping::new(),
            region_mapping: region_mapping::RegionMapping::new(),
        }
    }

    pub fn register_key(&mut self, key: Vec<String>, operation: Vec<String>) {
        self.key_mapping.register(key, operation);
    }

    pub fn register_mouse(&mut self, button: u32, region: Option<Region>, operation: Vec<String>) {
        self.mouse_mapping.register(button, region, operation);
    }

    pub fn register_event(&mut self, event_name: String, id: Option<String>, operation: Vec<String>) {
        self.event_mapping.register(event_name, id, operation);
    }

    pub fn register_region(&mut self, button: u32, operation: Vec<String>) {
        self.region_mapping.register(button, operation);
    }

    pub fn matched(&mut self, input: &Input, width: i32, height: i32) -> Vec<Vec<String>> {
        match *input {
            Input::Key(ref key) => {
                self.key_input_history.push(key.clone(), self.key_mapping.depth);
                self.key_mapping.matched(&self.key_input_history).into_iter().collect()
            }
            Input::MouseButton((x, y), ref button) =>
                self.mouse_mapping.matched(*button, x, y, width, height).into_iter().collect(),
            Input::Event(ref event_name) =>
                self.event_mapping.matched(event_name),
            Input::Region(_, button, _) =>
                self.region_mapping.matched(button).into_iter().collect(),
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
            Input::MouseButton(ref position, ref button) => format!("{:?}, {}", position, button),
            Input::Event(ref event_name) => o!(event_name),
            Input::Region(ref region, button, _) => format!("{}, {}", region, button),
        }
    }

    pub fn type_name(&self) -> &str {
        match *self {
            Input::Key(_) => "key",
            Input::MouseButton(_, _) => "mouse_button",
            Input::Event(_) => "event",
            Input::Region(_, _, _) => "region",
        }
    }
}
