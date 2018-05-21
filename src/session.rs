
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use app::App;
use color::Color;
use constant;
use entry::filter::expression::Expr as FilterExpr;
use entry::filter::writer::write as write_expr;
use entry::{Entry, EntryContainer, EntryType, Key, Meta};
use gui::Gui;
use mapping::{Mapping, input_mapping as imap, region_mapping as rmap, event_mapping as emap};
use operation::option::PreDefinedOptionName;
use option::common::{bool_to_str as b2s};
use paginator::Paginator;
use size::FitTo;
use state::{self, States, Filters};
use timer::TimerManager;
use util::path::path_to_str;
use util::shell::{escape, escape_pathbuf};
use util::time::duration_to_seconds;




#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Session {
    Options,
    Entries,
    Queue,
    Position,
    Paths,
    Mappings,
    Markers,
    Envs,
    Filter,
    Timers,
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
        Queue => write_queue(&app.remote_cache.state, out),
        Paths => write_paths(&app.entries, out),
        Position => write_paginator(app.current().map(|it| it.0), &app.paginator, out),
        Mappings => write_mappings(&app.mapping, out),
        Markers => write_markers(&app.marker, out),
        Envs => write_envs(out),
        Filter => write_filters(&app.states.last_filter, out),
        Timers => write_timers(&app.timers, out),
        Reading => {
            write_options(&app.states, &app.gui, true, out);
            write_entries(&app.entries, out);
            write_queue(&app.remote_cache.state, out);
            write_mappings(&app.mapping, out);
            write_markers(&app.marker, out);
            write_paginator(app.current().map(|it| it.0), &app.paginator, out);
        }
        All => {
            write_options(&app.states, &app.gui, false, out);
            write_entries(&app.entries, out);
            write_queue(&app.remote_cache.state, out);
            write_mappings(&app.mapping, out);
            write_markers(&app.marker, out);
            write_envs(out);
            write_filters(&app.states.last_filter, out);
            write_timers(&app.timers, out);
            write_paginator(app.current().map(|it| it.0), &app.paginator, out);
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

    fn gend(name: &str, value: &Duration, context: WriteContext) -> (String, Option<String>) {
        (gen_name(name, context), Some(s!(duration_to_seconds(value))))
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
        Animation => gen("animation", &b2s(st.drawing.animation), context),
        AutoReload => gen("auto-reload", &b2s(st.auto_reload), context),
        AutoPaging => gen("auto-paging", &st.auto_paging, context),
        ColorError => gen("error-color", &c2s(&gui.colors.error), context),
        ColorErrorBackground => gen("error-background-color", &c2s(&gui.colors.error_background), context),
        ColorLink => gen("link-color", &c2s(&st.drawing.link_color), context),
        CurlConnectTimeout => geno("curl-connect-timeout", &st.curl_options.connect_timeout, context),
        CurlFollowLocation => gen("curl-follow-location", &b2s(st.curl_options.follow_location), context),
        CurlLowSpeedLimit => geno("curl-low-speed-limit", &st.curl_options.low_speed_limit, context),
        CurlLowSpeedTime => geno("curl-low-speed-time", &st.curl_options.low_speed_time, context),
        CurlTimeout => geno("curl-timeout", &st.curl_options.connect_timeout, context),
        FitTo => gen("fit-to", &st.drawing.fit_to, context),
        HistoryFile => genp("history-file", &st.history_file, context),
        HorizontalViews => gen("horizontal-views", &st.view.cols, context),
        IdleTime => gend("idle-time", &st.idle_time, context),
        InitialPosition => gen("initial-position", &st.initial_position, context),
        LogFile => gen("log-file", &st.log_file, context),
        MaskOperator => gen("mask-operator", &st.drawing.mask_operator, context),
        OperationBox => gen("operation-box", &b2s(st.operation_box), context),
        PathList => gen("path", &st.path_list, context),
        PreFetchEnabled => gen("pre-render", &b2s(st.pre_fetch.enabled), context),
        PreFetchLimit => gen("pre-render-limit", &st.pre_fetch.limit_of_items, context),
        PreFetchPageSize => gen("pre-render-pages", &st.pre_fetch.page_size, context),
        Reverse => gen("reverse", &b2s(st.reverse), context),
        Rotation => gen("rotation", &st.drawing.rotation, context),
        SkipResizeWindow => gen("skip-resize-window", &st.skip_resize_window, context),
        StablePush => gen("stable-push", &b2s(st.stable_push), context),
        StatusBar => gen("status-bar", &b2s(st.status_bar), context),
        StatusBarAlign => gen("status-bar-align", &st.status_bar_align, context),
        StatusBarHeight => geno("status-bar-height", &st.status_bar_height, context),
        StatusFormat => gen("status-format", &st.status_format, context),
        StdOut => gen("stdout", &st.stdout, context),
        Style => gen("style", &st.style, context),
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

pub fn write_queue(state: &Arc<Mutex<::remote_cache::State>>, out: &mut String) {
    use entry::EntryType::*;

    let state = state.lock().unwrap();
    let requests = state.requests();
    for request in requests {
        let entry_type = request.entry_type.and_then(|entry_type| {
            let entry_type = match entry_type {
                PDF => "pdf",
                Image => "image",
                Archive => "archive",
                _ => return None,
            };
            Some(entry_type)
        });
        sprint!(out, "@push-url");
        if let Some(entry_type) = entry_type {
            sprint!(out, " --as {}", entry_type);
        }
        sprint!(out, "{}", meta_args(&request.meta));
        sprintln!(out, " {}", escape(&request.url));
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
                sprintln!(out, "@push-url --as image{} {}", meta_args(&entry.meta), escape(url)),
            Archive(_, _) if path_changed =>
                sprintln!(out, "@push-url --as archive{} {}", meta_args(&entry.meta), escape(url)),
            Pdf(_, _) if path_changed =>
                sprintln!(out, "@push-url --as pdf{} {}", meta_args(&entry.meta), escape(url)),
            Archive(_, _) | Pdf(_, _) | Memory(_, _) =>
                (),
        }
    } else {
        match entry.content {
            Image(ref path) =>
                sprintln!(out, "@push-image{} {}", meta_args(&entry.meta), escape_pathbuf(path)),
            Archive(ref path, _) if path_changed =>
                sprintln!(out, "@push-archive{} {}", meta_args(&entry.meta), escape_pathbuf(&*path)),
            Pdf(ref path, _) if path_changed =>
                sprintln!(out, "@push-pdf{} {}", meta_args(&entry.meta), escape_pathbuf(&*path)),
            Archive(_, _) | Pdf(_, _) | Memory(_, _) =>
                (),
        }
    }

    // To cut down the number of clone.
    if (previous.0.is_container() != entry.key.0.is_container()) || *previous.1 != entry.key.1 {
        *previous = entry.key.clone();
    }
}

fn meta_args(meta: &Option<Meta>) -> String {
    let mut result = o!("");
    if_let_some!(meta = meta.as_ref(), result);
    for entry in meta.iter() {
        sprint!(result, " --meta {}={}", escape(&entry.key), escape(&entry.value));
    }
    result
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

pub fn write_paginator(entry: Option<Arc<Entry>>, paginator: &Paginator, out: &mut String) {
    if let Some(entry) = entry {
        let (_, ref path, index) = entry.key;
        sprintln!(out, "@go {} {}", escape(path), index + 1);
    }
    sprintln!(out, "@fly-leaves {}", paginator.fly_leaves());
}

pub fn write_mappings(mappings: &Mapping, out: &mut String) {
    write_input_mappings(None, &mappings.input_mapping, out);
    write_region_mappings(&mappings.region_mapping, out);
    write_event_mappings(&mappings.event_mapping, out);
}

fn write_input_mappings(base: Option<&str>, mappings: &imap::InputMapping, out: &mut String) {
    for (name, entry) in &mappings.table {
        let name = if let Some(base) = base {
            format!("{},{}", base, name)
        } else {
            format!("{}", name)
        };
        write_input_mapping_entry(&name, entry, out);
    }
}

fn write_input_mapping_entry(name: &str, entry: &imap::Node, out: &mut String) {
    use self::imap::Node::*;

    match *entry {
        Sub(ref sub) =>
            write_input_mappings(Some(name), &*sub, out),
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

fn write_event_mappings(mappings: &emap::EventMapping, out: &mut String) {
    for (name, entries) in &mappings.table {
        for entry in entries.iter() {
            sprint!(out, "@map event");
            if let Some(group) = entry.group.as_ref() {
                sprint!(out, " --group {}", escape(group));
            }
            if let Some(remain) = entry.remain {
                if remain == 1 {
                    sprint!(out, " --once");
                } else {
                    sprint!(out, " --repeat {}", remain);
                }
            }
            sprint!(out, " {}", escape(&s!(name)));
            for it in &entry.operation {
                sprint!(out, " {}", escape(it));
            }
            sprintln!(out, "");

        }
    }
}

pub fn write_markers(marker: &HashMap<String, Key>, out: &mut String) {
    for (name, key) in marker {
        sprint!(out, "@mark {} {}", escape(name), escape(&key.1));
        sprint!(out, " {}", key.2 + 1);
        sprint!(out, " {}", key.0);
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
        let mut expr_text = o!("");
        write_expr(expr, &mut expr_text);
        sprint!(out, " {}", escape(&expr_text));
    }
    sprintln!(out, "");
}


pub fn write_timers(timers: &TimerManager, out: &mut String) {
    for (name, timer) in &timers.table {
        if !timer.is_live() {
            continue;
        }

        let interval = duration_to_seconds(&timer.interval);
        sprint!(out, "@timer --name {} --interval {}", escape(name), interval);
        if let Some(repeat) = timer.repeat {
            sprint!(out, " --repeat {}", repeat);
        } else {
            sprint!(out, " --infinity");
        }
        for it in &timer.operation {
            sprint!(out, " {}", escape(it));
        }
        sprintln!(out, "");
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

impl fmt::Display for state::Alignment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use gtk::Align::*;

        let result =
            match self.0 {
                Start => "start",
                Center => "center",
                End => "end",
                _ => panic!("Unexpected value: {:?}", self.0)
            };

        write!(f, "{}", result)
    }
}
