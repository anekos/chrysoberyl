
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use enum_iterator_derive::IntoEnumIterator;

use crate::app::App;
use crate::constant;
use crate::entry::filter::expression::Expr as FilterExpr;
use crate::entry::filter::writer::write as write_expr;
use crate::entry::{Entry, EntryContainer, EntryType, Key, Meta};
use crate::gui::{self, Gui};
use crate::mapping::{Mapping, input_mapping as imap, region_mapping as rmap, event_mapping as emap, operation_mapping as omap};
use crate::operation::option::PreDefinedOptionName;
use crate::option::common::{bool_to_str as b2s};
use crate::option::user_switch::{UserSwitch, UserSwitchManager};
use crate::paginator::Paginator;
use crate::size::FitTo;
use crate::state::{self, States, Filters};
use crate::timer::TimerManager;
use crate::util::path::path_to_str;
use crate::util::shell::{escape, escape_pathbuf};
use crate::util::time::duration_to_seconds;



#[derive(Clone, Debug, PartialEq, Copy, IntoEnumIterator)]
pub enum Session {
    Entries,
    Envs,
    Filter,
    Mappings,
    Markers,
    Options,
    Paths,
    Position,
    Queue,
    Reading,
    Status,
    Switches,
    Timers,
    All,
}

pub trait StatusText {
    fn write_status_text(&self, _: &mut String);
}


pub fn write_sessions(app: &App, sessions: &[Session], freeze: bool, out: &mut String) {
    if freeze {
        sprintln!(out, "@enable freeze");
    }

    for session in sessions {
        write_session(app, *session, out);
    }

    if freeze {
        sprintln!(out, "@queue @disable freeze");
    }
}

pub fn write_session(app: &App, session: Session, out: &mut String) {
    use self::Session::*;

    match session {
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
            write_filters(&app.states.last_filter, out);
            write_options(&app.states, &app.gui, true, out);
            write_entries(&app.entries, out);
            write_queue(&app.remote_cache.state, out);
            write_markers(&app.marker, out);
            write_paginator(app.current().map(|it| it.0), &app.paginator, out);
        },
        Status => write_status(&app, out),
        Switches => write_switches(&app.user_switches, out),
        All => {
            write_options(&app.states, &app.gui, false, out);
            write_switches(&app.user_switches, out);
            write_entries(&app.entries, out);
            write_queue(&app.remote_cache.state, out);
            write_mappings(&app.mapping, out);
            write_markers(&app.marker, out);
            write_envs(out);
            write_filters(&app.states.last_filter, out);
            write_timers(&app.timers, out);
            write_paginator(app.current().map(|it| it.0), &app.paginator, out);
            write_status(app, out);
        }
    }
}

pub fn generate_option_value(name: &PreDefinedOptionName, st: &States) -> (String, Option<String>) {
    use self::PreDefinedOptionName::*;

    fn gen<T: fmt::Display + Sized>(name: &str, value: &T) -> (String, Option<String>) {
        (o!(name), Some(s!(value)))
    }

    fn gend(name: &str, value: &Duration) -> (String, Option<String>) {
        (o!(name), Some(s!(duration_to_seconds(value))))
    }

    fn geno<T: fmt::Display + Sized>(name: &str, value: &Option<T>) -> (String, Option<String>) {
        (o!(name), value.as_ref().map(|it| s!(it)))
    }

    fn genp(name: &str, value: &Option<PathBuf>) -> (String, Option<String>) {
        if_let_some!(value = value.as_ref().and_then(|it| it.to_str()), (o!(name), None));
        (o!(name), Some(o!(value)))
    }

    match *name {
        AbbrevLength => gen("abbrev-length", &st.abbrev_length),
        Animation => gen("animation", &b2s(st.drawing.animation)),
        AutoReload => gen("auto-reload", &b2s(st.auto_reload)),
        AutoPaging => gen("auto-paging", &st.auto_paging),
        ColorLink => gen("link-color", &st.drawing.link_color),
        CurlConnectTimeout => geno("curl-connect-timeout", &st.curl_options.connect_timeout),
        CurlFollowLocation => gen("curl-follow-location", &b2s(st.curl_options.follow_location)),
        CurlLowSpeedLimit => geno("curl-low-speed-limit", &st.curl_options.low_speed_limit),
        CurlLowSpeedTime => geno("curl-low-speed-time", &st.curl_options.low_speed_time),
        CurlTimeout => geno("curl-timeout", &st.curl_options.connect_timeout),
        FitTo => gen("fit-to", &st.drawing.fit_to),
        Freeze => gen("freeze", &b2s(st.freezed)),
        HistoryFile => genp("history-file", &st.history_file),
        HorizontalFlip => gen("horizontal-flip", &st.drawing.horizontal_flip),
        HorizontalViews => gen("horizontal-views", &st.view.cols),
        IdleTime => gend("idle-time", &st.idle_time),
        IgnoreFailures => gen("ignore-failures", &b2s(st.ignore_failures)),
        InitialPosition => gen("initial-position", &st.initial_position),
        LogFile => gen("log-file", &st.log_file),
        MaskOperator => gen("mask-operator", &st.drawing.mask_operator),
        PathList => gen("path", &st.path_list),
        PreFetchEnabled => gen("pre-render", &b2s(st.pre_fetch.enabled)),
        PreFetchLimit => gen("pre-render-limit", &st.pre_fetch.limit_of_items),
        PreFetchPageSize => gen("pre-render-pages", &st.pre_fetch.page_size),
        PreFetchStages => gen("pre-render-stages", &st.pre_fetch.cache_stages),
        Reverse => gen("reverse", &b2s(st.reverse)),
        Rotation => gen("rotation", &st.drawing.rotation),
        Screen => gen("screen", &st.screen),
        SkipResizeWindow => gen("skip-resize-window", &st.skip_resize_window),
        StablePush => gen("stable-push", &b2s(st.stable_push)),
        StatusBar => gen("status-bar", &b2s(st.status_bar)),
        StatusBarAlign => gen("status-bar-align", &st.status_bar_align),
        StatusBarHeight => geno("status-bar-height", &st.status_bar_height),
        StatusFormat => gen("status-format", &st.status_format),
        StdOut => gen("stdout", &st.stdout),
        Style => gen("style", &st.style),
        EmptyStatusFormat => gen("empty-status-format", &st.empty_status_format),
        TimeToHidePointer => geno("time-to-hide-pointer", &st.time_to_hide_pointer),
        TitleFormat => gen("title-format", &st.title_format),
        UpdateCacheAccessTime => gen("update-cache-atime", &b2s(st.update_cache_atime)),
        VerticalFlip => gen("vertical-flip", &st.drawing.vertical_flip),
        VerticalViews => gen("vertical-views", &st.view.rows),
        WatchFiles => gen("watch-files", &b2s(st.watch_files)),
    }
}

pub fn write_options(st: &States, gui: &Gui, reading: bool, out: &mut String) {
    use self::PreDefinedOptionName::*;

    let write = |out: &mut String, name: &PreDefinedOptionName| {
        let (name, value) = generate_option_value(name, st);
        if let Some(value) = value {
            sprintln!(out, "@set {} {}", name, escape(&value));
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
            HorizontalViews | VerticalViews | Freeze => (),
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

pub fn write_queue(state: &Arc<Mutex<crate::remote_cache::State>>, out: &mut String) {
    use crate::entry::EntryType::*;

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

pub fn write_status(app: &App, out: &mut String) {
    app.cache.write_status_text(out);
    app.process_manager.write_status_text(out);
    app.remote_cache.write_status_text(out);
}

pub fn write_switches(switches: &UserSwitchManager, out: &mut String) {
    // FIXME Escape `@@`
    let mut switches: Vec<(&String, &UserSwitch)> = switches.iter().collect();
    switches.sort_by_key(|it| it.1);

    for (k, switch) in &switches {
        sprint!(out, "@define-switch {}", escape(k));
        for (index, op) in switch.iter().enumerate() {
            if index != 0 {
                sprint!(out, " @@");
            }
            for it in op {
                sprint!(out, " {}", escape(it));
            }
        }
        sprintln!(out, "");
        sprintln!(out, "@set {} {}", escape(k), switch.current_value());
    }
}

pub fn write_entries(entries: &EntryContainer, out: &mut String) {
    let mut previous = (EntryType::Invalid, o!(""), 0);
    for entry in entries.iter() {
        write_entry(entry, out, &mut previous);
    }
}

fn write_entry(entry: &Entry, out: &mut String, previous: &mut Key) {
    use crate::entry::EntryContent::*;

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
            Message(_) =>
                panic!("WTF"),
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
            Message(ref message) =>
                sprintln!(out, "@push-message{} {}", meta_args(&entry.meta), escape(message)),
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
    use crate::entry::EntryContent::*;

    if let Some(ref url) = entry.url {
        match entry.content {
            Image(_) =>
                out.push_str(&*url),
            Archive(_, ref entry) if entry.index == 0 =>
                out.push_str(&*url),
            Pdf(_, index) if index == 0 =>
                out.push_str(&*url),
            Archive(_, _) | Pdf(_, _) | Memory(_, _) | Message(_) =>
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
            Archive(_, _) | Pdf(_, _) | Memory(_, _) | Message(_) =>
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
    write_operation_mappings(&mappings.operation_mapping, out);
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

fn write_operation_mappings(mappings: &omap::OperationMapping, out: &mut String) {
    for (name, op) in &mappings.table {
        sprint!(out, "@map operation {}", escape(name));
        for it in op {
            sprint!(out, " {}", escape(it));
        }
        sprintln!(out, "");
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
        use crate::size::FitTo::*;

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

impl fmt::Display for gui::Screen {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::gui::Screen::*;

        let result =
            match *self {
                Main => "main",
                LogView => "log-view",
                CommandLine => "command-line",
                UserUI => "ui",
            };

        write!(f, "{}", result)
    }
}


impl fmt::Display for Session {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Session::*;

        let result =
            match *self {
                Entries => "entries",
                Envs => "envs",
                Filter => "filter",
                Mappings => "mappings",
                Markers => "markers",
                Options => "options",
                Paths => "paths",
                Position => "position",
                Queue => "queue",
                Reading => "reading",
                Status => "status",
                Switches => "switches",
                Timers => "timers",
                All => "all",
            };

        write!(f, "{}", result)
    }
}
