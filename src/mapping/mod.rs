
use gdk;

use events::EventName;
use gtk_wrapper::ScrollDirection;
use key::Key;
use size::Region;

pub mod event_mapping;
pub mod key_mapping;
pub mod mouse_mapping;
pub mod region_mapping;
pub mod wheel_mapping;



#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Event(EventName),
    Key(String),
    MouseButton((i32, i32), Key), // (X, Y), Button
    Region(Region, u32, usize), // region, button, cell_index
    Wheel(ScrollDirection),
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
    pub wheel_mapping: wheel_mapping::WheelMapping,
}


impl Mapping {
    pub fn new() -> Mapping {
        Mapping {
            key_input_history: key_mapping::KeyInputHistory::new(),
            key_mapping: key_mapping::KeyMapping::new(),
            mouse_mapping: mouse_mapping::MouseMapping::new(),
            event_mapping: event_mapping::EventMapping::new(),
            region_mapping: region_mapping::RegionMapping::new(),
            wheel_mapping: wheel_mapping::WheelMapping::new(),
        }
    }

    pub fn register_key(&mut self, key: Vec<String>, operation: Vec<String>) {
        self.key_mapping.register(key, operation);
    }

    pub fn register_mouse(&mut self, button: Key, region: Option<Region>, operation: Vec<String>) {
        self.mouse_mapping.register(button, region, operation);
    }

    pub fn register_event(&mut self, event_name: EventName, group: Option<String>, remain: Option<usize>, operation: Vec<String>) {
        self.event_mapping.register(event_name, group, remain, operation);
    }

    pub fn register_region(&mut self, button: u32, operation: Vec<String>) {
        self.region_mapping.register(button, operation);
    }

    pub fn register_wheel(&mut self, direction: ScrollDirection, operation: Vec<String>) {
        self.wheel_mapping.register(direction, operation);
    }

    pub fn unregister_key(&mut self, key: Vec<String>) {
        self.key_mapping.unregister(key);
    }

    pub fn unregister_mouse(&mut self, button: &Key, region: &Option<Region>) {
        self.mouse_mapping.unregister(button, region);
    }

    pub fn unregister_event(&mut self, event_name: &Option<EventName>, group: &Option<String>) {
        self.event_mapping.unregister(event_name, group);
    }

    pub fn unregister_region(&mut self, button: &u32) {
        self.region_mapping.unregister(button);
    }

    pub fn unregister_wheel(&mut self, direction: ScrollDirection) {
        self.wheel_mapping.unregister(direction);
    }

    pub fn matched(&mut self, input: &Input, width: i32, height: i32, decrease_remain: bool) -> Vec<Vec<String>> {
        let found = match *input {
            Input::Key(ref key) => {
                self.key_input_history.push(key.clone(), self.key_mapping.depth);
                self.key_mapping.matched(&self.key_input_history).into_iter().collect()
            }
            Input::MouseButton((x, y), ref button) =>
                self.mouse_mapping.matched(button.clone(), x, y, width, height).into_iter().collect(),
            Input::Event(ref event_name) =>
                self.event_mapping.matched(event_name, decrease_remain),
            Input::Region(_, button, _) =>
                self.region_mapping.matched(button).into_iter().collect(),
            Input::Wheel(direction) =>
                self.wheel_mapping.matched(direction).into_iter().collect(),
        };

        if found.is_empty() {
            return vec!();
        }

        found
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

    pub fn mouse_button(x: i32, y: i32, button: Key) -> Input {
        Input::MouseButton((x, y), button)
    }

    pub fn text(&self) -> String {
        match *self {
            Input::Key(ref name) => o!(name),
            Input::MouseButton(ref position, ref button) => format!("{:?}, {}", position, button),
            Input::Event(ref event_name) => s!(event_name),
            Input::Region(ref region, button, _) => format!("{}, {}", region, button),
            Input::Wheel(direction) => format!("{}", direction),
        }
    }

    pub fn type_name(&self) -> &str {
        match *self {
            Input::Key(_) => "key",
            Input::MouseButton(_, _) => "mouse_button",
            Input::Event(_) => "event",
            Input::Region(_, _, _) => "region",
            Input::Wheel(_) => "wheel",
        }
    }
}
