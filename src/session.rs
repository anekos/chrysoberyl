
use std::fmt;
use std::path::PathBuf;
use std::env;

use app::App;
use color::Color;
use constant;
use entry::filter::expression::Expr as FilterExpr;
use entry::filter::writer::write as write_expr;
use entry::{Entry, EntryContainer, EntryType, Key};
use gui::Gui;
use mapping::{Mapping, unified_mapping as umap, region_mapping as rmap};
use operation::option::PreDefinedOptionName;
use option::common::{bool_to_str as b2s};
use paginator::Paginator;
use size::FitTo;
use state::{self, States, Filters};
use util::path::path_to_str;
use util::shell::{escape, escape_pathbuf};




#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Session {
    Options,
    Entries,
    Position,
    Paths,
    Mappings,
    Envs,
    Filter,
    Reading,
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
        Options => write_options(&app.states, &app.gui, false, out),
        Entries => write_entries(&app.entries, out),
        Paths => write_paths(&app.entries, out),
        Position => write_paginator(&app.current().map(|it| it.0), &app.paginator, out),
        Mappings => write_mappings(&app.mapping, out),
        Envs => write_envs(out),
        Filter => write_filters(&app.states.last_filter, out),
        Reading => {
            write_options(&app.states, &app.gui, true, out);
            write_entries(&app.entries, out);
            write_mappings(&app.mapping, out);
            write_paginator(&app.current().map(|it| it.0), &app.paginator, out);
        }
        All => {
            write_options(&app.states, &app.gui, false, out);
            write_entries(&app.entries, out);
            write_mappings(&app.mapping, out);
            write_envs(out);
            write_filters(&app.states.last_filter, out);
            write_paginator(&app.current().map(|it| it.0), &app.paginator, out);
        }
    }
}

pub fn generate_option_value(name: &PreDefinedOptionName, st: &States, gui: &Gui, context: WriteContext) -> (String, Option<String>) {
    use self::PreDefinedOptionName::*;

    let esc = |s: &str| {
        match context {
            WriteContext::ENV => o!(s),
            WriteContext::Session => escape(s),
        }
    };

    let c2s = |c: &Color| {
        esc(&format!("{}", c))
    };

    fn gen_name(name: &str, context: WriteContext) -> String {
        match context {
            WriteContext::Session => o!(name),
            WriteContext::ENV => name.replace("-", "_").to_uppercase()
        }
    }

    fn gen<T: fmt::Display + Sized>(name: &str, value: &T, context: WriteContext) -> (String, Option<String>) {
        (gen_name(name, context), Some(s!(value)))
    }

    fn geno<T: fmt::Display + Sized>(name: &str, value: &Option<T>, context: WriteContext) -> (String, Option<String>) {
        (gen_name(name, context), value.as_ref().map(|it| s!(it)))
    }

    fn genp(name: &str, value: &Option<PathBuf>, context: WriteContext) -> (String, Option<String>) {
        if_let_some!(value = value.as_ref().and_then(|it| it.to_str()), (o!(name), None));
        (gen_name(name, context), Some(o!(value)))
    }

    match *name {
        AbbrevLength => gen("abbrev-length", &st.abbrev_length, context),
        AutoReload => gen("auto-reload", &b2s(st.auto_reload), context),
        AutoPaging => gen("auto-paging", &b2s(st.auto_paging), context),
        CenterAlignment => gen("center-alignment", &b2s(st.view.center_alignment), context),
        ColorError => gen("error-color", &c2s(&gui.colors.error), context),
        ColorErrorBackground => gen("error-background-color", &c2s(&gui.colors.error_background), context),
        ColorStatusBar => gen("status-bar-color", &c2s(&gui.colors.status_bar), context),
        ColorStatusBarBackground => gen("status-bar-background-color", &c2s(&gui.colors.status_bar_background), context),
        ColorWindowBackground => gen("window-background-color", &c2s(&gui.colors.window_background), context),
        CurlConnectTimeout => geno("curl-connect-timeout", &st.curl_options.connect_timeout, context),
        CurlFollowLocation => gen("curl-follow-location", &b2s(st.curl_options.follow_location), context),
        CurlLowSpeedLimit => geno("curl-low-speed-limit", &st.curl_options.low_speed_limit, context),
        CurlLowSpeedTime => geno("curl-low-speed-time", &st.curl_options.low_speed_time, context),
        CurlTimeout => geno("curl-timeout", &st.curl_options.connect_timeout, context),
        FitTo => gen("fit-to", &st.drawing.fit_to, context),
        HistoryFile => genp("history-file", &st.history_file, context),
        HorizontalViews => gen("horizontal-views", &st.view.cols, context),
        LogFile => gen("log-file", &st.log_file, context),
        MaskOperator => gen("mask-operator", &st.drawing.mask_operator, context),
        PathList => gen("path", &st.path_list, context),
        PreFetchEnabled => gen("pre-render", &b2s(st.pre_fetch.enabled), context),
        PreFetchLimit => gen("pre-render-limit", &st.pre_fetch.limit_of_items, context),
        PreFetchPageSize => gen("pre-render-pages", &st.pre_fetch.page_size, context),
        Reverse => gen("reverse", &b2s(st.reverse), context),
        Rotation => gen("rotation", &st.drawing.rotation, context),
        SkipResizeWindow => gen("skip-resize-window", &st.skip_resize_window, context),
        StatusBar => gen("status-bar", &b2s(st.status_bar), context),
        StatusBarHeight => geno("status-bar-height", &st.status_bar_height, context),
        StdOut => gen("stdout", &st.stdout, context),
        StatusFormat => gen("status-format", &st.status_format, context),
        EmptyStatusFormat => gen("empty-status-format", &st.empty_status_format, context),
        TitleFormat => gen("title-format", &st.title_format, context),
        UpdateCacheAccessTime => gen("update-cache-atime", &b2s(st.update_cache_atime), context),
        VerticalViews => gen("vertical-views", &st.view.rows, context),
        WatchFiles => gen("watch-files", &b2s(st.watch_files), context),
    }
}

pub fn write_options(st: &States, gui: &Gui, reading: bool, out: &mut String) {
    use self::PreDefinedOptionName::*;

    let write = |out: &mut String, name: &PreDefinedOptionName| {
        let (name, value) = generate_option_value(name, st, gui, WriteContext::Session);
        if let Some(value) = value {
            sprintln!(out, "@set {} {}", name, value);
        } else {
            sprintln!(out, "@unset {}", name);
        }
    };

    for option_name in PreDefinedOptionName::iterator() {
        if reading {
            match *option_name {
                Reverse | Rotation | StatusBar | FitTo => (),
                _ => continue,
            }
        }

        match *option_name {
            HorizontalViews | VerticalViews => (),
            _ => write(out, option_name),
        }
    }

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

    if let Some(ref url) = entry.url {
        let url = &*url;
        match entry.content {
            Image(_) =>
                sprintln!(out, "@push-url --as image {}", escape(url)),
            Archive(_, _) if path_changed =>
                sprintln!(out, "@push-url --as archive {}", escape(url)),
            Pdf(_, _) if path_changed =>
                sprintln!(out, "@push-url --as pdf {}", escape(url)),
            Archive(_, _) | Pdf(_, _) | Memory(_, _) =>
                (),
        }
    } else {
        match entry.content {
            Image(ref path) =>
                sprintln!(out, "@push-image {}", escape_pathbuf(path)),
            Archive(ref path, _) if path_changed =>
                sprintln!(out, "@push-archive {}", escape_pathbuf(&*path)),
            Pdf(ref path, _) if path_changed =>
                sprintln!(out, "@push-pdf {}", escape_pathbuf(&*path)),
            Archive(_, _) | Pdf(_, _) | Memory(_, _) =>
                (),
        }
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

    if let Some(ref url) = entry.url {
        match entry.content {
            Image(_) =>
                out.push_str(&*url),
            Archive(_, ref entry) if entry.index == 0 =>
                out.push_str(&*url),
            Pdf(_, index) if index == 0 =>
                out.push_str(&*url),
            Archive(_, _) | Pdf(_, _) | Memory(_, _) =>
                return,
        }
    } else {
        match entry.content {
            Image(ref path) =>
                out.push_str(path_to_str(&*path)),
            Archive(ref path, ref entry) if entry.index == 0 =>
                out.push_str(path_to_str(&**path)),
            Pdf(ref path, index) if index == 0 =>
                out.push_str(path_to_str(&**path)),
            Archive(_, _) | Pdf(_, _) | Memory(_, _) =>
                return,
        }
    }

    out.push_str("\n");
}

pub fn write_paginator(entry: &Option<Entry>, paginator: &Paginator, out: &mut String) {
    if let Some(ref entry) = *entry {
        let (_, ref path, index) = entry.key;
        sprintln!(out, "@go {} {}", escape(path), index + 1);
    }
    sprintln!(out, "@fly-leaves {}", paginator.fly_leaves());
}

pub fn write_mappings(mappings: &Mapping, out: &mut String) {
    write_unified_mappings(None, &mappings.unified_mapping, out);
    write_region_mappings(&mappings.region_mapping, out);
}

fn write_unified_mappings(base: Option<&str>, mappings: &umap::UnifiedMapping, out: &mut String) {
    for (name, entry) in &mappings.table {
        let name = if let Some(base) = base {
            format!("{},{}", base, name)
        } else {
            format!("{}", name)
        };
        write_unified_mapping_entry(&name, entry, out);
    }
}

fn write_unified_mapping_entry(name: &str, entry: &umap::Node, out: &mut String) {
    use self::umap::Node::*;

    match *entry {
        Sub(ref sub) =>
            write_unified_mappings(Some(name), &*sub, out),
        Leaf(ref leaf_node) => {
            for entry in &leaf_node.entries {
                sprint!(out, "@map input");
                if let Some(ref region) = entry.region {
                    sprint!(out, " --region {}x{}-{}x{}", region.left, region.top, region.right, region.bottom);
                }
                sprint!(out, " {}", escape(name));
                for it in &entry.operation {
                    sprint!(out, " {}", escape(it));
                }
                sprintln!(out, "");
            }
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

pub fn write_filters(filters: &Filters, out: &mut String) {
    write_filter(&filters.static_filter, " --static", out);
    write_filter(&filters.dynamic_filter, " --dynamic", out);
}

pub fn write_filter(expr: &Option<FilterExpr>, arg: &str, out: &mut String) {
    sprint!(out, "@filter{}", arg);
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
                Scale(scale) => return write!(f, "{}%", scale),
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
