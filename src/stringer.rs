
use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

use shell_escape;

use entry::{Entry, EntryContainer};
use gui::Gui;
use index_pointer::IndexPointer;
use mapping::{Mapping, key_mapping as kmap, mouse_mapping as mmap};
use size::FitTo;
use state::{States, ScalingMethod, RegionFunction};
use utils::path_to_str;



pub fn write_options(st: &States, gui: &Gui, out: &mut String) {
    fn b2s(b: bool) -> &'static str {
        if b {
            "true"
        } else {
            "false"
        }
    }

    sprintln!(out, "@set status-bar {}", b2s(st.status_bar));
    sprintln!(out, "@set auto-paging {}", b2s(st.auto_paging));
    sprintln!(out, "@set reverse {}", b2s(st.reverse));
    sprintln!(out, "@set status-format {}", escape(&st.status_format.0));
    sprintln!(out, "@set pre-render {}", b2s(st.pre_fetch.enabled));
    sprintln!(out, "@set pre-render-limit {}", st.pre_fetch.limit_of_items);
    sprintln!(out, "@set pre-render-pages {}", st.pre_fetch.page_size);
    sprintln!(out, "@set fit-to {}", st.drawing.fit_to);
    sprintln!(out, "@set scaling {}", st.drawing.scaling);
    sprintln!(out, "@set region-function {}", st.region_function);
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
    for entry in entries.iter() {
        write_entry(entry, out);
    }
}

fn write_entry(entry: &Entry, out: &mut String) {
    use entry::EntryContent::*;

    match entry.content {
        File(ref path) =>
            sprintln!(out, "@push-file {}", escape_pathbuf(path)),
        Http(_, ref url) =>
            sprintln!(out, "@push-url {}", escape(url)),
        Archive(ref path, ref entry) if entry.index == 0 =>
            sprintln!(out, "@push {}", escape_pathbuf(&*path)),
        Pdf(ref path, index) if index == 0 =>
            sprintln!(out, "@push-pdf {}", escape_pathbuf(&*path)),
        Archive(_, _) | Pdf(_, _) =>
            (),
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
            out.push_str(path_to_str(path)),
        Http(_, ref url) =>
            out.push_str(url),
        Archive(ref path, ref entry) if entry.index == 0 =>
            out.push_str(path_to_str(&*path)),
        Pdf(ref path, index) if index == 0 =>
            out.push_str(path_to_str(&*path)),
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
            if let Some(ref area) = entry.area {
                sprint!(out, " --area {}x{}-{}x{}", area.left, area.top, area.right, area.bottom);
            }
            sprint!(out, " {}", button);
            for it in &entry.operation {
                sprint!(out, " {}", escape(it));
            }
            sprintln!(out, "");
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


impl fmt::Display for RegionFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::RegionFunction::*;

        let result =
            match *self {
                Clip => "clip",
                Fill => "fill",
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
