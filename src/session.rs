
use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;
use std::env;

use shell_escape;

use app::App;
use color::Color;
use constant;
use entry::{Entry, EntryContainer, KeyType, Key};
use gui::Gui;
use index_pointer::IndexPointer;
use mapping::{Mapping, key_mapping as kmap, mouse_mapping as mmap, region_mapping as rmap};
use size::FitTo;
use state::{States, ScalingMethod};
use utils::path_to_str;



#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Session {
    Options,
    Entries,
    Position,
    Paths,
    Mappings,
    Envs,
    All,
}


pub fn write_sessions(app: &App, sessions: &[Session], out: &mut String) {
    for session in sessions {
        write_session(app, session, out);
    }
}

pub fn write_session(app: &App, session: &Session, out: &mut String) {
    use self::Session::*;

    match *session {
        Options => write_options(&app.states, &app.gui, out),
        Entries => write_entries(&app.entries, out),
        Paths => write_paths(&app.entries, out),
        Position => write_position(&app.entries, &app.pointer, out),
        Mappings => write_mappings(&app.mapping, out),
        Envs => write_envs(out),
        All => {
            write_options(&app.states, &app.gui, out);
            write_entries(&app.entries, out);
            write_position(&app.entries, &app.pointer, out);
            write_mappings(&app.mapping, out);
            write_envs(out);
        }
    }
}

pub fn write_options(st: &States, gui: &Gui, out: &mut String) {
    fn b2s(b: bool) -> &'static str {
        if b {
            "true"
        } else {
            "false"
        }
    }

    fn c2s(c: &Color) -> String {
        escape(&format!("{}", c))
    }

    sprintln!(out, "@set auto-paging {}", b2s(st.auto_paging));
    sprintln!(out, "@set center-alignment {}", b2s(st.view.center_alignment));
    sprintln!(out, "@set fit-to {}", st.drawing.fit_to);
    sprintln!(out, "@set pre-render {}", b2s(st.pre_fetch.enabled));
    sprintln!(out, "@set pre-render-limit {}", st.pre_fetch.limit_of_items);
    sprintln!(out, "@set pre-render-pages {}", st.pre_fetch.page_size);
    sprintln!(out, "@set reverse {}", b2s(st.reverse));
    sprintln!(out, "@set scaling {}", st.drawing.scaling);
    sprintln!(out, "@set status-bar {}", b2s(st.status_bar));
    sprintln!(out, "@set status-format {}", escape(&st.status_format.0));
    sprintln!(out, "@set title-format {}", escape(&st.title_format.0));
    sprintln!(out, "@set window-background-color {}", c2s(&gui.colors.window_background));
    sprintln!(out, "@set status-bar-color {}", c2s(&gui.colors.status_bar));
    sprintln!(out, "@set status-bar-background-color {}", c2s(&gui.colors.status_bar_background));
    sprintln!(out, "@set error-color {}", c2s(&gui.colors.error));
    sprintln!(out, "@set error-background-color {}", c2s(&gui.colors.error_background));
    // sprintln!(out, "@set horizontal-views {}", gui.cols());
    // sprintln!(out, "@set vertical-views {}", gui.rows());
    sprintln!(out, "@views {} {}", gui.cols(), gui.rows());
    if let Some(c) = st.drawing.clipping {
        sprintln!(out, "@clip {} {} {} {}", c.left, c.top, c.right, c.bottom);
    } else {
        sprintln!(out, "@unclip");
    }
}

pub fn write_entries(entries: &EntryContainer, out: &mut String) {
    let mut previous = (KeyType::Invalid, o!(""), 0);
    for entry in entries.iter() {
        write_entry(entry, out, &mut previous);
    }
}

fn write_entry(entry: &Entry, out: &mut String, previous: &mut Key) {
    use entry::EntryContent::*;

    let path_changed = previous.1 != entry.key.1;

    match entry.content {
        File(ref path) =>
            sprintln!(out, "@push-image {}", escape_pathbuf(path)),
        Http(_, ref url) =>
            sprintln!(out, "@push-url {}", escape(url)),
        Archive(ref path, _) if path_changed =>
            sprintln!(out, "@push {}", escape_pathbuf(&*path)),
        Pdf(ref path, _) if path_changed =>
            sprintln!(out, "@push-pdf {}", escape_pathbuf(&*path)),
        Archive(_, _) | Pdf(_, _) =>
            (),
    }

    // To cut down the number of clone.
    if (previous.0.is_container() != entry.key.0.is_container()) || *previous.1 != entry.key.1 {
        *previous = entry.key.clone();
    }
}

pub fn write_paths(entries: &EntryContainer, out: &mut String) {
    for entry in entries.iter() {
        write_path(entry, out);
    }
}

fn write_path(entry: &Entry, out: &mut String) {
    use entry::EntryContent::*;

    match entry.content {
        File(ref path) =>
            out.push_str(path_to_str(&*path)),
        Http(_, ref url) =>
            out.push_str(url),
        Archive(ref path, ref entry) if entry.index == 0 =>
            out.push_str(path_to_str(&**path)),
        Pdf(ref path, index) if index == 0 =>
            out.push_str(path_to_str(&**path)),
        Archive(_, _) | Pdf(_, _) =>
            (),
    }
}

pub fn write_position(entries: &EntryContainer, pointer: &IndexPointer, out: &mut String) {
    if let Some((entry, _)) = entries.current(pointer) {
        let (_, ref path, index) = entry.key;
        sprintln!(out, "@show {} {}", escape(path), index + 1);
    }
}

pub fn write_mappings(mappings: &Mapping, out: &mut String) {
    write_key_mappings(None, &mappings.key_mapping, out);
    write_mouse_mappings(&mappings.mouse_mapping, out);
    write_region_mappings(&mappings.region_mapping, out);
}

fn write_key_mappings(base: Option<&str>, mappings: &kmap::KeyMapping, out: &mut String) {
    for (name, entry) in &mappings.table {
        let name = if let Some(base) = base {
            format!("{},{}", base, name)
        } else {
            o!(name)
        };
        write_key_mapping_entry(&name, entry, out);
    }
}

fn write_key_mapping_entry(name: &str, entry: &kmap::MappingEntry, out: &mut String) {
    use self::kmap::MappingEntry::*;

    match *entry {
        Sub(ref sub) =>
            write_key_mappings(Some(name), &*sub, out),
        Code(ref code) => {
            sprint!(out, "@map key {}", escape(name));
            for it in code {
                sprint!(out, " {}", escape(it));
            }
            sprintln!(out, "");
        }
    }
}

fn write_mouse_mappings(mappings: &mmap::MouseMapping, out: &mut String) {
    for (button, entries) in &mappings.table {
        for entry in entries {
            sprint!(out, "@map mouse");
            if let Some(ref region) = entry.region {
                sprint!(out, " --region {}x{}-{}x{}", region.left, region.top, region.right, region.bottom);
            }
            sprint!(out, " {}", button);
            for it in &entry.operation {
                sprint!(out, " {}", escape(it));
            }
            sprintln!(out, "");
        }
    }
}

fn write_region_mappings(mappings: &rmap::RegionMapping, out: &mut String) {
    for (button, operation) in &mappings.table {
        sprint!(out, "@map region {} ", button);
        for it in operation {
            sprint!(out, " {}", escape(it));
        }
        sprintln!(out, "");
    }
}

fn write_envs(out: &mut String) {
    for (key, value) in env::vars_os() {
        if let (Ok(ref key), Ok(ref value)) = (key.into_string(), value.into_string()) {
            if key.starts_with(constant::USER_VARIABLE_PREFIX) {
                let key = &key[constant::USER_VARIABLE_PREFIX.len()..];
                sprintln!(out, "@set-env -p {} {}", escape(key), escape(value));
            }
        }
    }
}


impl fmt::Display for FitTo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use size::FitTo::*;

        let result =
            match *self {
                Original => "original",
                OriginalOrCell => "original-or-cell",
                Width => "width",
                Height => "height",
                Cell => "cell",
                Fixed(w, h) => return write!(f, "{}x{}", w, h),
            };

        write!(f, "{}", result)
    }
}


impl fmt::Display for ScalingMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use gdk_pixbuf::InterpType::*;

        let result = match self.0 {
            Nearest => "nearest",
            Tiles => "tiles",
            Bilinear => "bilinear",
            Hyper => "hyper",
        };
        write!(f, "{}", result)
    }
}


fn escape_pathbuf(path: &PathBuf) -> String {
    escape(path_to_str(path))
}

fn escape(s: &str) -> String {
    let s = Cow::from(o!(s));
    shell_escape::escape(s).into_owned()
}
