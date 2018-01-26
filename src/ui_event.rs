
extern crate gio_sys;
extern crate gobject_sys;
extern crate glib_sys;

use std::cell::Cell;
use std::default::Default;
use std::error::Error;
use std::ffi::{CString, CStr};
use std::mem::transmute;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Duration;

use enum_primitive::FromPrimitive;
use gdk::{EventButton, EventKey, EventConfigure, EventScroll, ScrollDirection};
use gtk::prelude::*;
use gtk::{Inhibit, SelectionData};
use libc::c_void;
use self::gio_sys::{g_file_new_for_uri, g_file_get_path, GFile};
use self::glib_sys::g_free;
use self::gobject_sys::{GObject, g_object_unref};

use events::EventName;
use expandable::Expandable;
use gui::{Gui, DropItemType};
use key::{Key, Coord};
use lazy_sender::LazySender;
use mapping::Input;
use operation::Operation;
use util::num::feq;



#[derive(Clone, Copy, Default)]
struct Conf {
    width: u32,
    height: u32,
    spawned: bool,
    skip: usize,
}


type ArcPressedAt = Arc<Cell<Option<(f64, f64)>>>;
type ArcConf = Arc<Cell<Conf>>;


pub fn register(gui: &Gui, skip: usize, tx: &Sender<Operation>) {
    let sender = LazySender::new(tx.clone(), Duration::from_millis(50));
    let pressed_at = Arc::new(Cell::new(None));
    let conf = Arc::new(Cell::new(Conf { skip: skip, .. Conf::default() }));

    gui.window.connect_key_press_event(clone_army!([tx] move |_, key| on_key_press(&tx, key)));
    gui.window.connect_configure_event(clone_army!([conf, tx, sender] move |_, ev| on_configure(sender.clone(), &tx, ev, &conf)));
    gui.window.connect_delete_event(clone_army!([tx] move |_, _| on_delete(&tx)));
    gui.window.connect_button_press_event(clone_army!([pressed_at] move |_, button| on_button_press(button, &pressed_at)));
    gui.window.connect_button_release_event(clone_army!([conf, tx] move |_, button| on_button_release(&tx, button, &pressed_at, &conf)));
    gui.window.connect_scroll_event(clone_army!([tx] move |_, scroll| on_scroll(&tx, scroll)));

    gui.vbox.connect_drag_data_received(clone_army!([tx] move |_, _, _, _, selection, info, _| {
        if let Some(drop_item_type) = DropItemType::from_u32(info) {
            on_drag_data_received(&tx, selection, &drop_item_type)
        }
    }));
}


fn on_key_press(tx: &Sender<Operation>, key: &EventKey) -> Inhibit {
    let keyval = key.as_ref().keyval;
    if 48 <= keyval && keyval <= 57 {
        tx.send(Operation::CountDigit((keyval - 48) as u8)).unwrap();
    } else if !is_modifier_key(key.get_keyval()) {
        let key = Key::from(key);
        tx.send(Operation::Input(Input::Unified(Coord::default(), key))).unwrap();
    }

    Inhibit(false)
}

fn on_button_press(button: &EventButton, pressed_at: &ArcPressedAt) -> Inhibit {
    let (x, y) = button.get_position();
    (*pressed_at).set(Some((x, y)));
    Inhibit(true)
}

fn on_button_release(tx: &Sender<Operation>, button: &EventButton, pressed_at: &ArcPressedAt, conf: &ArcConf) -> Inhibit {
    let c = conf.get();
    let (x, y) = button.get_position();
    if_let_some!((px, py) = (*pressed_at).get(), Inhibit(true));
    if feq(x, px, 10.0) && feq(y, py, 10.0) {
        tx.send(
            Operation::Input(
                Input::Unified(Coord { x: x as i32, y: y as i32, width: c.width, height: c.height }, Key::from(button)))).unwrap();
    } else {
        tx.send(Operation::TellRegion(px, py, x, y, Key::from(button))).unwrap();
    }
    Inhibit(true)
}

fn on_configure(mut sender: LazySender, tx: &Sender<Operation>, ev: &EventConfigure, conf: &ArcConf) -> bool {
    let (w, h) = ev.get_size();
    let mut c = conf.get();

    trace!("configure: w={} h={} lw={} lh={}", w, h, c.width, c.height);

    if c.width == w && c.height == h {
        return false;
    }

    if 0 < c.skip {
        c.skip -= 1;
        trace!("on_configure/skip: remain={:?}", c.skip);
    } else if c.spawned {
        sender.request(EventName::ResizeWindow.operation());
    } else {
        tx.send(EventName::Spawn.operation()).unwrap();
        c.spawned = true;
    }
    c.width = w;
    c.height = h;

    conf.set(c);

    false
}

fn on_delete(tx: &Sender<Operation>) -> Inhibit {
    tx.send(EventName::Quit.operation()).unwrap();
    Inhibit(false)
}

fn on_scroll(tx: &Sender<Operation>, scroll: &EventScroll) -> Inhibit {
    if scroll.get_direction() != ScrollDirection::Smooth {
        tx.send(Operation::Input(Input::Unified(Coord::default(), Key::from(scroll)))).unwrap();
    }
    Inhibit(true)
}

fn on_drag_data_received(tx: &Sender<Operation>, selection: &SelectionData, drop_item_type: &DropItemType) {
    match *drop_item_type {
        DropItemType::Path => {
            for uri in &selection.get_uris() {
                match uri_to_path(uri) {
                    Ok(path) => tx.send(Operation::Push(Expandable(path), None, false)).unwrap(),
                    Err(err) => puts_error!(err),
                }
            }
        },
        DropItemType::URI => {
            if let Some(url) = selection.get_text() {
                tx.send(Operation::PushURL(url, None, false, None)).unwrap();
            }
        }
    }
}

fn uri_to_path(uri: &str) -> Result<String, Box<Error>> {
    let uri = CString::new(uri).unwrap();

    unsafe {
        let g_file = g_file_new_for_uri(uri.into_raw());
        let c_path = g_file_get_path(g_file);
        let path = CStr::from_ptr(c_path);
        let path = path.to_str()?.to_string();

        let ptr = transmute::<*const GFile, *mut GObject>(g_file);
        g_object_unref(ptr);
        let ptr = transmute::<*mut i8, *mut c_void>(c_path);
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
