
extern crate gio_sys;
extern crate gobject_sys;
extern crate glib_sys;

use std::default::Default;
use std::error::Error;
use std::ffi::{CString, CStr};
use std::mem::transmute;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::time::Duration;

use enum_primitive::FromPrimitive;
use gdk::ScrollDirection;
use gtk::prelude::*;
use gtk::{Inhibit, SelectionData};
use libc::c_void;
use self::gio_sys::{g_file_new_for_uri, g_file_get_path, GFile};
use self::glib_sys::g_free;
use self::gobject_sys::{GObject, g_object_unref};

use events::EventName;
use expandable::Expandable;
use gui::{Gui, DropItemType, Screen};
use key::Key;
use lazy_sender::LazySender;
use mapping::Mapped;
use operation::Operation;
use size::CoordPx;
use util::num::feq;



pub struct UIEvent {
    tx: Sender<Event>,
}


#[derive(Clone, Copy, Default)]
struct Conf {
    width: u32,
    height: u32,
    spawned: bool,
    skip: usize,
}


pub enum Event {
    ButtonPress((f64, f64)),
    ButtonRelease(Key, (f64, f64)),
    Configure((u32, u32)),
    Delete,
    Motion,
    Scroll(Key, ScrollDirection),
    UIKeyPress(Key),
    UpdateScreen(Screen),
    UpdateTimeToHidePointer(Option<u32>),
    WindowKeyPress(Key, u32),
}


impl UIEvent {
    pub fn new(gui: &Gui, skip: usize, app_tx: &Sender<Operation>) -> Self {
        UIEvent { tx: register(gui, skip, app_tx) }
    }

    pub fn update_screen(&self, screen: Screen) {
        self.tx.send(Event::UpdateScreen(screen)).unwrap();
    }

    pub fn update_time_to_hide_pointer(&self, time: Option<u32>) {
        self.tx.send(Event::UpdateTimeToHidePointer(time)).unwrap();
    }
}

fn register(gui: &Gui, skip: usize, app_tx: &Sender<Operation>) -> Sender<Event> {
    use self::Event::*;

    let (tx, rx) = channel();

    gui.operation_entry.connect_key_press_event(clone_army!([tx] move |_, key| {
        tx.send(UIKeyPress(Key::from(key))).unwrap();
        Inhibit(false)
    }));

    gui.log_view.connect_key_press_event(clone_army!([tx] move |_, key| {
        tx.send(UIKeyPress(Key::from(key))).unwrap();
        Inhibit(false)
    }));

    gui.vbox.connect_key_press_event(clone_army!([tx] move |_, key| {
        tx.send(WindowKeyPress(Key::from(key), key.as_ref().keyval)).unwrap();
        Inhibit(true)
    }));

    gui.event_box.connect_motion_notify_event(clone_army!([tx] move |_, _| {
        tx.send(Motion).unwrap();
        Inhibit(false)
    }));

    gui.event_box.connect_button_press_event(clone_army!([tx] move |_, button| {
        tx.send(ButtonPress(button.get_position())).unwrap();
        Inhibit(true)
    }));

    gui.event_box.connect_button_release_event(clone_army!([tx] move |_, button| {
        tx.send(ButtonRelease(Key::from(button), button.get_position())).unwrap();
        Inhibit(true)
    }));

    gui.window.connect_configure_event(clone_army!([tx] move |_, ev| {
        tx.send(Configure(ev.get_size())).unwrap();
        false
    }));

    gui.window.connect_delete_event(clone_army!([tx] move |_, _| {
        tx.send(Delete).unwrap();
        Inhibit(true)
    }));

    gui.window.connect_scroll_event(clone_army!([tx] move |_, scroll| {
        tx.send(Scroll(Key::from(scroll), scroll.get_direction())).unwrap();
        Inhibit(true)
    }));

    gui.overlay.connect_drag_data_received(clone_army!([app_tx] move |_, _, _, _, selection, info, _| {
        if let Some(drop_item_type) = DropItemType::from_u32(info) {
            on_drag_data_received(&app_tx, selection, &drop_item_type)
        }
    }));

    thread::spawn(clone_army!([app_tx] move || main(&app_tx, &rx, skip)));

    tx
}

fn main(app_tx: &Sender<Operation>, rx: &Receiver<Event>, skip: usize) {
    use self::Event::*;
    use gui::{Screen as S};

    let mut on_configure_sender = LazySender::new(app_tx.clone(), Duration::from_millis(50));
    let mut on_motion_sender = LazySender::new(app_tx.clone(), Duration::from_millis(1000));
    let mut conf = Conf { skip, .. Conf::default() };
    let mut pressed_at = None;
    let mut screen = S::Main;
    let mut ignore_motion = false;

    while let Ok(event) = rx.recv() {
        match event {
            UIKeyPress(ref key) =>
                entry_on_ui(app_tx, key),
            WindowKeyPress(key, keyval) =>
                match screen {
                    S::Main | S::UserUI =>  on_key_press(app_tx, key, keyval),
                    _ => (),
                },
            ButtonPress((x, y)) if screen == S::Main =>
                pressed_at = Some((x, y)),
            ButtonRelease(key, (x, y)) =>
                match screen {
                    S::Main => on_button_release(app_tx, key, x, y, &mut pressed_at, &mut conf),
                    S::UserUI => on_button_release_on_user_ui(app_tx),
                    _ => (),
                },
            Delete =>
                app_tx.send(EventName::Quit.operation()).unwrap(),
            Configure((w, h)) =>
                on_configure(&mut on_configure_sender, app_tx, w, h, &mut conf),
            UpdateTimeToHidePointer(None) => {
                app_tx.send(Operation::Pointer(true)).unwrap();
                on_motion_sender.cancel();
                ignore_motion = true;
            },
            UpdateTimeToHidePointer(Some(0)) => {
                ignore_motion = true;
                app_tx.send(Operation::Pointer(false)).unwrap();
            }
            UpdateTimeToHidePointer(Some(time)) => {
                on_motion_sender.set_delay(Duration::from_millis(time.into()));
                on_motion(&mut on_motion_sender);
                ignore_motion = false;
            },
            UpdateScreen(new_screen) =>
                screen = new_screen,
            Scroll(key, direction) =>
                if screen == S::Main { on_scroll(app_tx, key, direction) },
            Motion if !ignore_motion =>
                on_motion(&mut on_motion_sender),
            _ => (),
        }
    }
}

fn entry_on_ui(tx: &Sender<Operation>, key: &Key) {
    use operation::UIActionType::*;

    let action_type = match key.as_str() {
        "Return" => SendOperation,
        "Escape" => Close,
        _ => return,
    };

    tx.send(Operation::UIAction(action_type)).unwrap();
}

fn on_key_press(tx: &Sender<Operation>, key: Key, keyval: u32) {
    use gdk::enums::key;

    if key::_0 <= keyval && keyval <= key::_9 {
        tx.send(Operation::CountDigit((keyval - key::_0) as u8)).unwrap();
    } else if !is_modifier_key(keyval) {
        tx.send(Operation::Fire(Mapped::Input(CoordPx::default(), key))).unwrap();
    }
}

fn on_button_release(tx: &Sender<Operation>, key: Key, x: f64, y: f64, pressed_at: &mut Option<(f64, f64)>, conf: &mut Conf) {
    if_let_some!((px, py) = *pressed_at);

    if feq(x, px, 10.0) && feq(y, py, 10.0) {
        tx.send(
            Operation::Fire(
                Mapped::Input(CoordPx { x: x as i32, y: y as i32, width: conf.width, height: conf.height }, key))).unwrap();
    } else {
        tx.send(Operation::TellRegion(px, py, x, y, key)).unwrap();
    }
}

fn on_button_release_on_user_ui(tx: &Sender<Operation>) {
    use operation::option::OptionName::PreDefined;
    use operation::option::OptionUpdater::Unset;
    use operation::option::PreDefinedOptionName::Screen;

    tx.send(
        Operation::UpdateOption(PreDefined(Screen), Unset)).unwrap();
}

fn on_configure(sender: &mut LazySender, tx: &Sender<Operation>, w: u32, h: u32, conf: &mut Conf) {
    if conf.width == w && conf.height == h {
        return;
    }

    if 0 < conf.skip {
        conf.skip -= 1;
    } else if conf.spawned {
        sender.request(EventName::ResizeWindow.operation());
    } else {
        tx.send(EventName::Spawn.operation()).unwrap();
        conf.spawned = true;
    }
    conf.width = w;
    conf.height = h;
}

fn on_motion(sender: &mut LazySender) {
    sender.initialize(Operation::Pointer(true));
    sender.request(Operation::Pointer(false));
}

fn on_drag_data_received(tx: &Sender<Operation>, selection: &SelectionData, drop_item_type: &DropItemType) {
    match *drop_item_type {
        DropItemType::Path => {
            for uri in &selection.get_uris() {
                match uri_to_path(uri) {
                    Ok(path) => tx.send(Operation::Push(Expandable::expanded(path), None, false, false)).unwrap(),
                    Err(err) => puts_error!(err),
                }
            }
        },
        DropItemType::URI => {
            if let Some(url) = selection.get_text() {
                tx.send(Operation::PushURL(url, None, false, false, None)).unwrap();
            }
        }
    }
}

fn on_scroll(app_tx: &Sender<Operation>, key: Key, direction: ScrollDirection) {
    if direction != ScrollDirection::Smooth {
        app_tx.send(Operation::Fire(Mapped::Input(CoordPx::default(), key))).unwrap();
    }
}

fn uri_to_path(uri: &str) -> Result<String, Box<Error>> {
    let uri = CString::new(uri).unwrap();

    unsafe {
        let g_file = g_file_new_for_uri(uri.into_raw());
        let c_path = g_file_get_path(g_file);
        let path = CStr::from_ptr(c_path);
        let path = path.to_str()?.to_string();

        #[allow(clippy::transmute_ptr_to_ptr)]
        let ptr = transmute::<*const GFile, *mut GObject>(g_file);
        g_object_unref(ptr);
        let ptr = c_path as *mut c_void;
        g_free(ptr);

        Ok(path)
    }
}

fn is_modifier_key(key: u32) -> bool {
    use gdk::enums::key;

    match key {
        key::Shift_L | key::Shift_R | key::Control_L | key::Control_R | key::Meta_L | key::Meta_R | key::Alt_L | key::Alt_R | key::Super_L | key::Super_R | key::Hyper_L | key::Hyper_R =>
            true,
        _ =>
            false,
    }
}
