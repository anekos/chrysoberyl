
use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;
use std::env;

use shell_escape;

use app::App;
use color::Color;
use constant;
use entry::filter::expression::Expr as FilterExpr;
use entry::filter::writer::write as write_expr;
use entry::{Entry, EntryContainer, EntryType, Key};
use gui::Gui;
use mapping::{Mapping, key_mapping as kmap, mouse_mapping as mmap, region_mapping as rmap};
use operation::PreDefinedOptionName;
use size::FitTo;
use state::{self, States, ScalingMethod};
use utils::path_to_str;



#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Session {
    Options,
    Entries,
    Position,
    Paths,
    Mappings,
    Envs,
    Filter,
    All,
}

#[derive(Clone, Debug, Copy)]
pub enum WriteContext {
    Session,
    ENV
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
        Position => write_paginator(&app.current_entry(), out),
        Mappings => write_mappings(&app.mapping, out),
        Envs => write_envs(out),
        Filter => write_filter(&app.states.last_filter, out),
        All => {
            write_options(&app.states, &app.gui, out);
            write_entries(&app.entries, out);
            write_mappings(&app.mapping, out);
            write_envs(out);
            write_filter(&app.states.last_filter, out);
            write_paginator(&app.current_entry(), out);
        }
    }
}

pub fn generate_option_value(name: &PreDefinedOptionName, st: &States, gui: &Gui, context: WriteContext) -> (String, String) {
    use self::PreDefinedOptionName::*;

    let esc = |s: &str| {
        match context {
            WriteContext::ENV => o!(s),
            WriteContext::Session => escape(s),
        }
    };

    fn b2s(b: bool) -> &'static str {
        if b {
            "true"
        } else {
            "false"
        }
    }

    let c2s = |c: &Color| {
        esc(&format!("{}", c))
    };

    fn gen<T: fmt::Display + Sized>(name: &str, value: &T, context: WriteContext) -> (String, String) {
        let name = match context {
            WriteContext::Session => o!(name),
            WriteContext::ENV => name.replace("-", "_").to_uppercase()
        };;
        (o!(name), s!(value))
    }

    match *name {
        AutoPaging => gen("auto-paging", &b2s(st.auto_paging), context),
        CenterAlignment => gen("center-alignment", &b2s(st.view.center_alignment), context),
        ColorError => gen("error-color", &c2s(&gui.colors.error), context),
        ColorErrorBackground => gen("error-background-color", &c2s(&gui.colors.error_background), context),
        ColorStatusBar => gen("status-bar-color", &c2s(&gui.colors.status_bar), context),
        ColorStatusBarBackground => gen("status-bar-background-color", &c2s(&gui.colors.status_bar_background), context),
        ColorWindowBackground => gen("window-background-color", &c2s(&gui.colors.window_background), context),
        FitTo => gen("fit-to", &st.drawing.fit_to, context),
        HorizontalViews => gen("horizontal-views", &st.view.cols, context),
        LogFile => gen("log-file", &st.log_file, context),
        MaskOperator => gen("mask-operator", &st.drawing.mask_operator, context),
        PreFetchEnabled => gen("pre-render", &b2s(st.pre_fetch.enabled), context),
        PreFetchLimit => gen("pre-render-limit", &st.pre_fetch.limit_of_items, context),
        PreFetchPageSize => gen("pre-render-pages", &st.pre_fetch.page_size, context),
        Reverse => gen("reverse", &b2s(st.reverse), context),
        Scaling => gen("scaling", &st.drawing.scaling, context),
        StatusBar => gen("status-bar", &b2s(st.status_bar), context),
        StatusFormat => gen("status-format", &esc(&st.status_format.0), context),
        TitleFormat => gen("title-format", &esc(&st.title_format.0), context),
        VerticalViews => gen("vertical-views", &st.view.rows, context),
    }
}

pub fn write_options(st: &States, gui: &Gui, out: &mut String) {
    use self::PreDefinedOptionName::*;

    let write = |out: &mut String, name: PreDefinedOptionName| {
        let (name, value) = generate_option_value(&name, st, gui, WriteContext::Session);
        sprintln!(out, "@set {} {}", name, value);
    };

    write(out, AutoPaging);
    write(out, CenterAlignment);
    write(out, ColorError);
    write(out, ColorErrorBackground);
    write(out, ColorStatusBar);
    write(out, ColorStatusBarBackground);
    write(out, ColorWindowBackground);
    write(out, FitTo);
    // write(out, HorizontalViews);
    write(out, MaskOperator);
    write(out, PreFetchEnabled);
    write(out, PreFetchLimit);
    write(out, PreFetchPageSize);
    write(out, Reverse);
    write(out, Scaling);
    write(out, StatusBar);
    write(out, StatusFormat);
    write(out, TitleFormat);
    // write(out, VerticalViews);

    sprintln!(out, "@views {} {}", gui.cols(), gui.rows());
    if let Some(c) = st.drawing.clipping {
        sprintln!(out, "@clip {} {} {} {}", c.left, c.top, c.right, c.bottom);
    } else {
        sprintln!(out, "@unclip");
    }
}

pub fn write_entries(entries: &EntryContainer, out: &mut String) {
    let mut previous = (EntryType::Invalid, o!(""), 0);
    for entry in entries.iter() {
        write_entry(entry, out, &mut previous);
    }
}

fn write_entry(entry: &Entry, out: &mut String, previous: &mut Key) {
    use entry::EntryContent::*;

    let path_changed = previous.1 != entry.key.1;

    match entry.content {
        Image(ref path) =>
            sprintln!(out, "@push-image {}", escape_pathbuf(path)),
        Archive(ref path, _) if path_changed =>
            sprintln!(out, "@push-archive {}", escape_pathbuf(&*path)),
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
        Image(ref path) =>
            out.push_str(path_to_str(&*path)),
        Archive(ref path, ref entry) if entry.index == 0 =>
            out.push_str(path_to_str(&**path)),
        Pdf(ref path, index) if index == 0 =>
            out.push_str(path_to_str(&**path)),
        Archive(_, _) | Pdf(_, _) =>
            (),
    }
}

pub fn write_paginator(entry: &Option<Entry>, out: &mut String) {
    if let Some(ref entry) = *entry {
        let (_, ref path, index) = entry.key;
        sprintln!(out, "@go {} {}", escape(path), index + 1);
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

pub fn write_filter(expr: &Option<FilterExpr>, out: &mut String) {
    sprint!(out, "@filter");
    if let Some(ref expr) = *expr {
        sprint!(out, " ");
        write_expr(expr, out);
    }
    sprintln!(out, "");
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


impl fmt::Display for state::MaskOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use cairo::Operator::*;

        let result =
            match self.0 {
                Clear => "clear",
                Source => "source",
                Over => "over",
                In => "in",
                Out => "out",
                Atop => "atop",
                Dest => "dest",
                DestOver => "dest-over",
                DestIn => "dest-in",
                DestOut => "dest-out",
                DestAtop => "dest-atop",
                Xor => "xor",
                Add => "add",
                Saturate => "saturate",
                Multiply => "multiply",
                Screen => "screen",
                Overlay => "overlay",
                Darken => "darken",
                Lighten => "lighten",
                ColorDodge => "color-dodge",
                ColorBurn => "color-burn",
                HardLight => "hard-light",
                SoftLight => "soft-light",
                Difference => "difference",
                Exclusion => "exclusion",
                HslHue => "hsl-hue",
                HslSaturation => "hsl-saturation",
                HslColor => "hsl-color",
                HslLuminosity => "hsl-luminosity",
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
