
use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

use shell_escape;

use entry::{Entry, EntryContainer};
use index_pointer::IndexPointer;
use size::FitTo;
use state::{States, ScalingMethod};
use utils::path_to_str;



pub fn write_options(st: &States, out: &mut String) {
    sprintln!(out, "@set status-bar {}", b2s(st.status_bar));
    sprintln!(out, "@set auto-paging {}", b2s(st.auto_paging));
    sprintln!(out, "@set reverse {}", b2s(st.auto_paging));
    sprintln!(out, "@set status-format {}", escape(&st.status_format.0));
    sprintln!(out, "@set pre-render {}", b2s(st.pre_fetch.enabled));
    sprintln!(out, "@set pre-render-limit {}", st.pre_fetch.limit_of_items);
    sprintln!(out, "@set pre-render-pages {}", st.pre_fetch.page_size);
    sprintln!(out, "@set fit-to {}", st.drawing.fit_to);
    sprintln!(out, "@set scaling {}", st.drawing.scaling);
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

pub fn write_position(entries: &EntryContainer, pointer: &IndexPointer, out: &mut String) {
    if let Some((entry, _)) = entries.current(pointer) {
        let (_, ref path, index) = entry.key;
        sprintln!(out, "@show {} {}", escape(path), index + 1);
    }
}

fn b2s(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
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


fn escape_pathbuf(path: &PathBuf) -> String {
    escape(path_to_str(path))
}

fn escape(s: &str) -> String {
    let s = Cow::from(o!(s));
    shell_escape::escape(s).into_owned()
}
