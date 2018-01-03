
use std::env;
use std::fs::File;
use std::io::{Write, Read};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;

use gtk::prelude::*;
use natord;
use rand::distributions::{IndependentSample, Range as RandRange};

use app_path;
use archive;
use cherenkov::fill::Shape;
use color::Color;
use command_line;
use config::DEFAULT_CONFIG;
use constant::VARIABLE_PREFIX;
use editor;
use entry::filter::expression::Expr as FilterExpr;
use entry::{Meta, SearchKey, Entry, EntryContent, EntryType};
use errors::ChryError;
use events::EventName;
use expandable::{Expandable, expand_all};
use file_extension::get_entry_type_from_filename;
use filer;
use fragile_input::new_fragile_input;
use gui::Direction;
use key::Key;
use logger;
use operation::option::{OptionName, OptionUpdater};
use operation::{self, Operation, OperationContext, MappingTarget, MoveBy};
use option::user_switch::DummySwtich;
use poppler::{PopplerDocument, self};
use script;
use session::{Session, write_sessions};
use shell;
use shell_filter;
use state;
use util::num::range_contains;
use util::path::path_to_str;

use app::*;



pub fn on_app_event(app: &mut App, updated: &mut Updated, event_name: &EventName, context: &HashMap<String, String>) {
    use self::EventName::*;

    let async = match *event_name {
        Spawn => true,
        _ => false,
    };

    trace!("on_app_event: event={}, async={}", event_name, async);

    match *event_name {
        ResizeWindow => on_window_resized(app, updated),
        Initialize => on_initialized(app),
        Spawn => on_spawn(app),
        _ => ()
    }

    let op = Operation::Input(Input::Event(event_name.clone()));
    if async {
        app.tx.send(op).unwrap();
    } else {
        for (k, v) in context {
            env::set_var(constant::env_name(k), v);
        }
        app.operate(op);
    }

    if *event_name == Quit {
        on_quit();
    }
}

pub fn on_cherenkov(app: &mut App, updated: &mut Updated, parameter: &operation::CherenkovParameter, context: Option<OperationContext>) {
    use cherenkov::{Che, Modifier};
    use cherenkov::nova::Nova;

    if let Some(Input::Unified(coord, _)) = context.map(|it| it.input) {
        let cell_size = app.gui.get_cell_size(&app.states.view, app.states.status_bar);

        for (index, cell) in app.gui.cells(app.states.reverse).enumerate() {
            if let Some((entry, _)) = app.current_with(index) {
                let (x1, y1, w, h) = {
                    let (cx, cy, cw, ch) = cell.get_top_left();
                    if let Some((iw, ih)) = cell.get_image_size() {
                        (cx + (cw - iw) / 2, cy + (ch - ih) / 2, iw, ih)
                    } else {
                        continue;
                    }
                };
                let (x2, y2) = (x1 + w, y1 + h);
                if x1 <= coord.x && coord.x <= x2 && y1 <= coord.y && coord.y <= y2 {
                    let center = (
                        f64!(parameter.x.unwrap_or_else(|| coord.x - x1)) / f64!(w),
                        f64!(parameter.y.unwrap_or_else(|| coord.y - y1)) / f64!(h));
                    app.cache.cherenkov1(
                        &entry,
                        &cell_size,
                        Modifier {
                            search_highlight: false,
                            che: Che::Nova(Nova {
                                center: center,
                                n_spokes: parameter.n_spokes,
                                radius: parameter.radius,
                                random_hue: parameter.random_hue,
                                color: parameter.color,
                            })
                        },
                        &app.states.drawing);
                    updated.image = true;
                }
            }
        }
    }
}

pub fn on_clear(app: &mut App, updated: &mut Updated) {
    app.entries.clear();
    app.paginator.reset();
    app.cache.clear();
    updated.image = true;
}

pub fn on_clip(app: &mut App, updated: &mut Updated, inner: Region, context: Option<OperationContext>) {
    let inner = extract_region_from_context(context).map(|it| it.0).unwrap_or(inner);
    let current = app.states.drawing.clipping.unwrap_or_default();
    app.states.drawing.clipping = Some(current + inner);
    updated.image_options = true;
}


pub fn on_initial_process(app: &mut App, entries: Vec<command_line::Entry>, shuffle: bool) {
    use command_line::{Entry as CLE};

    app.reset_view();

    app.update_label_visibility();

    let mut first_path = None;

    {
        let mut updated = Updated::default();
        for entry in entries {
            match entry {
                CLE::Path(file) => {
                    if first_path.is_none() {
                        first_path = Some(file.clone());
                    }
                    on_events::on_push(app, &mut updated, file.clone(), None, false);
                }
                CLE::Input(file) => {
                    controller::register_file(app.tx.clone(), file);
                },
                CLE::Expand(file, recursive) => {
                    on_events::on_push(app, &mut updated, file.clone(), None, false);
                    app.tx.send(Operation::Expand(recursive, Some(Path::new(&file).to_path_buf()))).unwrap();
                },
                CLE::Operation(op) => {
                    match Operation::parse_from_vec(&op) {
                        Ok(op) => app.tx.send(op).unwrap(),
                        Err(err) => puts_error!(err, "at" => "operation", "for" => join(&op, ' ')),
                    }
                }
            }
        }
    }

    controller::register_stdin(app.tx.clone(), app.states.history_file.clone());

    if shuffle {
        let fix = first_path.map(|it| Path::new(&it).is_file()).unwrap_or(false);
        app.tx.send(Operation::Shuffle(fix)).unwrap();
    }

    app.initialize_envs_for_options();
    app.update_paginator_condition();

    app.tx.send(EventName::Initialize.operation()).unwrap();
}


pub fn on_editor(app: &mut App, editor_command: Option<Expandable>, files: &[Expandable], sessions: &[Session]) {
    let tx = app.tx.clone();
    let source = with_ouput_string!(out, {
        for file in files {
            if let Err(err) = File::open(file.expand()).and_then(|mut file| file.read_to_string(out)) {
                puts_error!(err, "at" => o!("on_load"));
            }
        }
        write_sessions(app, sessions, out);
    });
    spawn(move || editor::start_edit(&tx, editor_command.map(|it| it.to_string()), &source));
}

pub fn on_error(app: &mut App, updated: &mut Updated, error: String) {
    if app.error_loop_detector.in_loop(&error) {
        return;
    }

    env::set_var(constant::env_name("ERROR"), &error);
    app.update_message(Some(error));
    updated.message = true;
    app.fire_event(EventName::Error);
}

pub fn on_expand(app: &mut App, updated: &mut Updated, recursive: bool, base: Option<PathBuf>) {
    let count = app.counter.pop();
    let center = app.current_for_file();
    let serial = app.store();
    let app_info = app.app_info();

    let expanded = if recursive {
        app.entries.expand(&app_info, center, base, 1, count as u8)
    } else {
        app.entries.expand(&app_info, center, base, count as u8, count as u8- 1)
    };

    app.update_paginator_condition();

    if expanded {
        app.restore_or_first(updated, serial);
    }

    updated.label = true;
}

pub fn on_define_switch(app: &mut App, name: String, values: Vec<Vec<String>>) {
    match app.user_switches.register(name, values) {
        Ok(op) => app.operate(op),
        Err(error) => puts_error!(error, "at" => "on_define_switch"),
    }
}

pub fn on_delete(app: &mut App, updated: &mut Updated, expr: FilterExpr) {
    let current_index = app.paginator.current_index();
    let app_info = app.app_info();

    let after_index = app.entries.delete(&app_info, current_index, Box::new(move |ref mut entry, app_info| expr.evaluate(entry, app_info)));

    if let Some(after_index) = after_index {
        app.paginator.update_index(Index(after_index));
    } else {
        app.paginator.reset_level();
    }

    app.update_paginator_condition();

    updated.pointer = true;
    updated.image = true;
    updated.message = true;
}

#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
pub fn on_fill(app: &mut App, updated: &mut Updated, shape: Shape, region: Option<Region>, color: Color, mask: bool, cell_index: usize, context: Option<OperationContext>) {
    use cherenkov::{Modifier, Che};

    let (region, cell_index) = extract_region_from_context(context)
        .or_else(|| region.map(|it| (it, cell_index)))
        .unwrap_or_else(|| (Region::full(), cell_index));

    if let Some((entry, _)) = app.current_with(cell_index) {
        let cell_size = app.gui.get_cell_size(&app.states.view, app.states.status_bar);
        app.cache.cherenkov1(
            &entry,
            &cell_size,
            Modifier {
                search_highlight: false,
                che: Che::Fill(shape, region, color, mask),
            },
            &app.states.drawing);
        updated.image = true;
    }
}

pub fn on_filter(app: &mut App, updated: &mut Updated, dynamic: bool, expr: Option<FilterExpr>) {
    if dynamic {
        app.states.last_filter.dynamic_filter = expr.clone();
    } else {
        app.states.last_filter.static_filter = expr.clone();
    }

    let app_info = app.app_info();
    let current_index = app.paginator.current_index();
    let after_index = if let Some(expr) = expr {
        app.entries.update_filter(&app_info, dynamic, current_index, Some(Box::new(move |ref mut entry, app_info| expr.evaluate(entry, app_info))))
    } else {
        app.entries.update_filter(&app_info, dynamic, current_index, None)
    };

    app.update_paginator_condition();

    if let Some(after_index) = after_index {
        app.paginator.update_index(Index(after_index));
    } else {
        app.paginator.reset_level();
    }

    updated.pointer = true;
    updated.image = true;
    updated.message = true;

    app.update_message(Some(o!("Done")));
}

pub fn on_first(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy) {
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(false, ignore_views, count);
            updated.pointer = app.paginator.first(paging);
        },
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).pop();
            if let Some(first) = app.entries.find_nth_archive(count, false) {
                let paging = app.paging_with_index(false, ignore_views, first);
                updated.pointer = app.paginator.show(paging);
            }
        }
    }
}

pub fn on_fly_leaves(app: &mut App, updated: &mut Updated, n: usize) {
    updated.pointer = app.paginator.set_fly_leaves(n);
}

pub fn on_fragile(app: &mut App, path: &Expandable) {
    new_fragile_input(app.tx.clone(), &path.expand());
}

pub fn on_go(app: &mut App, updated: &mut Updated, key: &SearchKey) {
    let index = app.entries.search(key);
    if let Some(index) = index {
        if app.paginator.update_index(Index(index)) {
            updated.pointer = true;
            return;
        }
    }

    app.states.go = Some(key.clone());
}

pub fn on_initialized(app: &mut App) {
    app.tx.send(Operation::UpdateUI).unwrap();

    ui_event::register(&app.gui, app.states.skip_resize_window, &app.primary_tx.clone());
    app.gui.update_colors();
    app.update_label(true, true);
    app.gui.show();
}

pub fn on_input(app: &mut App, input: &Input) {
    let (width, height) = app.gui.window.get_size();

    if_let_some!((operations, inputs) = app.mapping.matched(input, width, height, true), {
        puts_event!("input", "type" => input.type_name(), "name" => s!(input));
    });

    for op in operations {
        match Operation::parse_from_vec(&op) {
            Ok(op) =>
                app.operate(Operation::Context(OperationContext { input: input.clone(), cell_index: None }, Box::new(op))),
            Err(err) =>
                puts_error!(err, "at" => "input")
        }
    }

    if let Input::Unified(coord, _) = *input {
        let context = convert_args!(hashmap!("input" => inputs, "x" => s!(coord.x), "y" => s!(coord.y)));
        app.fire_event_with_context(EventName::MappedInput, context);
    }
}

pub fn on_kill_timer(app: &mut App, name: &str) {
    app.timers.unregister(name);
}

pub fn on_last(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy) {
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(false, ignore_views, count);
            updated.pointer = app.paginator.last(paging);
        }
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).pop();
            if let Some(nth) = app.entries.find_nth_archive(count, true) {
                let paging = app.paging_with_index(false, ignore_views, nth);
                updated.pointer = app.paginator.show(paging);
            }
        }
    }
}

pub fn on_lazy_draw(app: &mut App, updated: &mut Updated, to_end: &mut bool, serial: u64, new_to_end: bool) {
    trace!("on_lazy_draw: draw_serial={} serial={}", app.draw_serial, serial);
    if app.draw_serial == serial {
        if app.do_clear_cache {
            puts_event!("on_lazy_draw/clear_cache");
            app.cache.clear();
            app.do_clear_cache = false;
        }
        updated.image = true;
        *to_end = new_to_end;
    }
}

pub fn on_load(app: &mut App, file: &Expandable, search_path: bool) {
    let path = if search_path { file.search_path(&app.states.path_list) } else { file.expand() };
    script::load_from_file(&app.tx, &path, &app.states.path_list);
}

pub fn on_load_default(app: &mut App) {
    script::load(&app.tx, DEFAULT_CONFIG, &app.states.path_list);
}

pub fn on_make_visibles(app: &mut App, regions: &[Option<Region>]) {
    app.gui.make_visibles(regions);
}

pub fn on_map(app: &mut App, target: MappingTarget, remain: Option<usize>, operation: Vec<String>) {
    use app::MappingTarget::*;

    // puts_event!("map", "target" => format!("{:?}", target), "operation" => format!("{:?}", operation));
    match target {
        Unified(key_sequence, region) =>
            app.mapping.register_unified(key_sequence, region, operation),
        Event(Some(event_name), group) =>
            app.mapping.register_event(event_name, group, remain, operation),
        Event(None, _) =>
            panic!("WTF"),
        Region(button) =>
            app.mapping.register_region(button, operation),
    }
}

#[allow(unused_variables)]
pub fn on_meow(app: &mut App, updated: &mut Updated) {
    /* for develop */
    updated.image = false;
}

#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
pub fn on_move_again(app: &mut App, updated: &mut Updated, to_end: &mut bool, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool) {
    if app.states.last_direction == state::Direction::Forward {
        on_next(app, updated, count, ignore_views, move_by, wrap)
    } else {
        on_previous(app, updated, to_end, count, ignore_views, move_by, wrap)
    }
}

pub fn on_multi(app: &mut App, mut operations: VecDeque<Operation>, async: bool) {
    if async {
        if let Some(op) = operations.pop_front() {
            app.operate(op);
        }
        if !operations.is_empty() {
            app.tx.send(Operation::Multi(operations, async)).unwrap();
        }
    } else {
        for op in operations {
            app.operate(op);
        }
    }
}

pub fn on_next(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool) {
    app.states.last_direction = state::Direction::Forward;
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(wrap, ignore_views, count);
            updated.pointer = app.paginator.next(paging);
        }
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).pop();
            let current = app.current();
            if let Some(next) = app.entries.find_next_archive(current, count) {
                let paging = app.paging_with_index(false, ignore_views, next);
                updated.pointer = app.paginator.show(paging);
            }
        }
    }
}

pub fn on_operate_file(app: &mut App, file_operation: &filer::FileOperation) {
    use entry::EntryContent::*;
    use archive::ArchiveEntry;

    if let Some((entry, _)) = app.current() {
        let result = match entry.content {
            Image(ref path) => file_operation.execute(path),
            Archive(_ , ArchiveEntry { ref content, .. }) => {
                let name = entry.page_filename();
                file_operation.execute_with_buffer(content, &name)
            },
            Pdf(ref path, index) => {
                let name = entry.page_filename();
                let png = PopplerDocument::new_from_file(&**path).nth_page(index).get_png_data(&file_operation.size);
                file_operation.execute_with_buffer(png.as_ref(), &name)
            },
        };
        let text = format!("{:?}", file_operation);
        match result {
            Ok(_) => puts_event!("operate_file", "status" => "ok", "operation" => text),
            Err(err) => puts_error!(err, "at" => "operate_file", "status" => "fail", "operation" => text),
        }
    }
}

pub fn on_page(app: &mut App, updated: &mut Updated, page: usize) {
    if_let_some!((_, index) = app.current(), ());
    if_let_some!(found = app.entries.find_page_in_archive(index, page), ());
    updated.pointer = app.paginator.update_index(Index(found));
}

pub fn on_pdf_index(app: &App, async: bool, read_operations: bool, search_path: bool, command_line: &[Expandable], fmt: &poppler::index::Format, separator: Option<&str>) {
    if_let_some!((entry, _) = app.current(), ());
    if let EntryContent::Pdf(path, _) = entry.content {
        let mut stdin = o!("");
        PopplerDocument::new_from_file(&*path).index().write(fmt, separator, &mut stdin);
        shell::call(async, &expand_all(command_line, search_path, &app.states.path_list), Some(stdin), option!(read_operations, app.tx.clone()));
    } else {
        puts_error!(ChryError::Fixed("current entry is not PDF"), "at" => "on_pdf_index");
    }
}

pub fn on_pre_fetch(app: &mut App, serial: u64) {
    let pre_fetch = app.states.pre_fetch.clone();
    if pre_fetch.enabled {
        trace!("on_pre_fetch: pre_fetch_serial={} serial={}", app.pre_fetch_serial, serial);

        if app.pre_fetch_serial == serial {
            let cell_size = app.gui.get_cell_size(&app.states.view, app.states.status_bar);
            app.pre_fetch(cell_size, 1..pre_fetch.page_size);
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
pub fn on_previous(app: &mut App, updated: &mut Updated, to_end: &mut bool, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool) {
    app.states.last_direction = state::Direction::Backward;
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(wrap, ignore_views, count);
            updated.pointer = app.paginator.previous(paging);
            *to_end = count.is_none() && !ignore_views;
        }
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).pop();
            let current = app.current();
            if let Some(previous) = app.entries.find_previous_archive(current, count) {
                let paging = app.paging_with_index(false, ignore_views, previous);
                updated.pointer = app.paginator.show(paging);
            }
        }
    }
}

pub fn on_pull(app: &mut App, updated: &mut Updated) {
    let buffered = app.sorting_buffer.pull_all();
    push_buffered(app, updated, buffered);
}

pub fn on_push(app: &mut App, updated: &mut Updated, path: String, meta: Option<Meta>, force: bool) {
    if is_url(&path) {
        app.tx.send(Operation::PushURL(path, meta, force, None)).unwrap();
        return;
    }

    on_push_path(app, updated, &Path::new(&path).to_path_buf(), meta, force)
}

pub fn on_push_archive(app: &mut App, path: &PathBuf, meta: Option<Meta>, force: bool, url: Option<String>) {
    archive::fetch_entries(path, meta, &app.encodings, app.tx.clone(), app.sorting_buffer.clone(), force, url);
}

pub fn on_push_path(app: &mut App, updated: &mut Updated, path: &PathBuf, meta: Option<Meta>, force: bool) {
    if let Ok(path) = path.canonicalize() {
        if let Some(entry_type) = get_entry_type_from_filename(&path) {
            match entry_type {
                EntryType::Archive =>
                    return on_push_archive(app, &path, meta, force, None),
                EntryType::PDF =>
                    return on_push_pdf(app, updated, path.to_path_buf(), meta, force, None),
                _ =>
                    ()
            }
        }
    }

    if path.is_dir() {
        on_push_directory(app, updated, path.clone(), meta, force)
    } else {
        on_push_image(app, updated, path.clone(), meta, force, None, None)
    }
}

pub fn on_push_directory(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool) {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushDirectory(file, meta, force));
    push_buffered(app, updated, buffered);
}

pub fn on_push_image(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool, expand_level: Option<u8>, url: Option<String>) {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushImage(file, meta, force, expand_level, url));
    push_buffered(app, updated, buffered);
}

pub fn on_push_pdf(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool, url: Option<String>) {
    let document = PopplerDocument::new_from_file(&file);
    let n_pages = document.n_pages();

    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushPdfEntries(file, n_pages, meta, force, url));
    push_buffered(app, updated, buffered);
}

pub fn on_push_sibling(app: &mut App, updated: &mut Updated, next: bool, meta: Option<Meta>, force: bool, go: bool) {
    fn find_sibling(base: &PathBuf, next: bool) -> Option<PathBuf> {
        base.parent().and_then(|dir| {
            dir.read_dir().ok().and_then(|dir| {
                let mut entries: Vec<PathBuf> = dir.filter_map(|it| it.ok()).filter(|it| it.file_type().map(|it| it.is_file()).unwrap_or(false)).map(|it| it.path()).collect();
                entries.sort_by(|a, b| natord::compare(path_to_str(a), path_to_str(b)));
                entries.iter().position(|it| it == base).and_then(|found| {
                    if next {
                        entries.get(found + 1).cloned()
                    } else if found > 0 {
                        entries.get(found - 1).cloned()
                    } else {
                        None
                    }
                })
            })
        })
    }

    use entry::EntryContent::*;

    let found = app.current().and_then(|(entry, _)| {
        match entry.content {
            Image(ref path) =>
                find_sibling(path, next),
            Archive(ref path, _) | Pdf(ref path, _) =>
                find_sibling(&*path, next),
        }
    });

    if let Some(found) = found {
        if go {
            on_go(app, updated, &SearchKey { path: o!(path_to_str(&found)), index: None});
        }
        on_push_path(app, updated, &found, meta, force);
    }
}

pub fn on_push_url(app: &mut App, updated: &mut Updated, url: String, meta: Option<Meta>, force: bool, entry_type: Option<EntryType>) {
    let buffered = app.remote_cache.fetch(url, meta, force, entry_type);
    push_buffered(app, updated, buffered);
}

pub fn on_quit() {
    termination::execute();
}

pub fn on_random(app: &mut App, updated: &mut Updated, len: usize) {
    if len > 0 {
        let index = RandRange::new(0, len).ind_sample(&mut app.rng);
        let paging = app.paging_with_index(false, false, index);
        app.paginator.show(paging);
        updated.image = true;
    }
}

pub fn on_reset_image(app: &mut App, updated: &mut Updated) {
    if let Some((entry, _)) = app.current() {
        app.cache.uncherenkov(&entry.key);
        updated.image_options = true;
    }
}

pub fn on_reset_scrolls(app: &mut App, to_end: bool) {
    app.gui.reset_scrolls(to_end);
}

pub fn on_save(app: &mut App, path: &Option<PathBuf>, sessions: &[Session]) {
    let default = app_path::config_file(Some(app_path::DEFAULT_SESSION_FILENAME));
    let path = path.as_ref().unwrap_or(&default);

    let result = File::create(path).map(|mut file| {
        file.write_all(with_ouput_string!(out, write_sessions(app, sessions, out)).as_str().as_bytes())
    });

    if let Err(err) = result {
        puts_error!(err, "at" => "save_session")
    }
}

pub fn on_search_text(app: &mut App, updated: &mut Updated, text: Option<String>, backward: bool, color: Color) {
    use cherenkov::{Che, Modifier};

    fn opt_range_contains(range: &Option<Range<usize>>, index: usize, if_none: bool) -> bool {
        range.as_ref().map(|it| range_contains(it, &index)).unwrap_or(if_none)
    }

    if let Some(text) = text {
        if text.trim() == "" {
            app.update_message(None);
            updated.message = true;
            return;
        }

        if app.cache.clear_search_highlights() {
            updated.image = true;
        }
        app.found_on = None;
        app.search_text = Some(text);
    } else if let Some(new_value) = app.found_on.as_ref().and_then(|found_on| {
        app.current().map(|(_, index)| index .. index + app.gui.len() - 1).and_then(|current| {
            if current != *found_on { Some(current) } else { None }
        })
    }) {
        app.found_on = Some(new_value);
    }

    updated.message = true;

    if_let_some!(text = app.search_text.clone(), app.update_message(Some(o!("Empty"))));

    let seq: Vec<(usize, Rc<Entry>)> = if backward {
        let skip = app.paginator.current_index().map(|index| app.entries.len() - index - 1).unwrap_or(0);
        app.entries.iter().cloned().enumerate().rev().skip(skip).collect()
    } else {
        let skip = app.paginator.current_index().unwrap_or(0);
        app.entries.iter().cloned().enumerate().skip(skip).collect()
    };

    let mut previous: Option<(Rc<PopplerDocument>, PathBuf)> = None;
    let mut new_found_on = None;
    let cells = app.gui.len();
    let mut first_regions = vec![];

    for (index, entry) in seq {
        if !opt_range_contains(&new_found_on, index, true) { break; }
        if opt_range_contains(&app.found_on, index, false) { continue; }

        if let EntryContent::Pdf(ref path, ref doc_index) = entry.content {
            let mut doc: Option<Rc<PopplerDocument>> = None;

            if let Some((ref p_doc, ref p_path)) = previous {
                if **path == *p_path {
                    doc = Some(p_doc.clone());
                }
            }

            if doc.is_none() {
                let d = Rc::new(PopplerDocument::new_from_file(&**path));
                doc = Some(d.clone());
                previous = Some((d, (**path).clone()));
            }

            let page = doc.unwrap().nth_page(*doc_index);
            let regions = page.find_text(&text);

            if regions.is_empty() {
                if new_found_on.is_some() {
                    first_regions.push(None);
                }
                continue;
            }
            first_regions.push(Some(regions[0]));

            let cell_size = app.gui.get_cell_size(&app.states.view, app.states.status_bar);

            app.cache.clear_entry_search_highlights(&entry);
            let modifiers: Vec<Modifier> = regions.iter().map(|region| Modifier { search_highlight: true, che: Che::Fill(Shape::Rectangle, *region, color, false) }).collect();
            app.cache.cherenkov(
                &entry,
                &cell_size,
                modifiers.as_slice(),
                &app.states.drawing);

            if new_found_on.is_none() {
                updated.pointer = app.paginator.update_index(Index(index));
                updated.image = true;
                app.update_message(Some(o!("Found!")));
                let left = index / cells * cells;
                new_found_on = Some(left .. left + cells - 1);
            }
        }
    }

    if new_found_on.is_none() {
        app.update_message(Some(o!("Not found!")));
    } else {
        updated.target_regions = Some(first_regions);
    }
    app.found_on = new_found_on;
}

pub fn on_set_env(_: &mut App, name: &str, value: &Option<String>) {
    if let Some(ref value) = *value {
        env::set_var(name, value);
    } else {
        env::remove_var(name);
    }
}

pub fn on_scroll(app: &mut App, direction: &Direction, operation: &[String], scroll_size: f64) {
    let saved = app.counter.clone();
    if !app.gui.scroll_views(direction, scroll_size, app.counter.pop()) && !operation.is_empty() {
        match Operation::parse_from_vec(operation) {
            Ok(op) => {
                app.counter = saved;
                app.operate(op);
            },
            Err(err) => puts_error!(err, "at" => "scroll"),
        }
    }
}

pub fn on_shell(app: &mut App, async: bool, read_operations: bool, search_path: bool, command_line: &[Expandable], sessions: &[Session]) {
    let stdin = if !sessions.is_empty() {
        Some(with_ouput_string!(out, write_sessions(app, sessions, out)))
    } else {
        None
    };

    set_count_env(app);
    let tx = app.tx.clone();
    shell::call(async, &expand_all(command_line, search_path, &app.states.path_list), stdin, option!(read_operations, tx));
}

pub fn on_shell_filter(app: &mut App, command_line: &[Expandable], search_path: bool) {
    set_count_env(app);
    shell_filter::start(expand_all(command_line, search_path, &app.states.path_list), app.tx.clone());
}

pub fn on_show(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy) {
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(false, false, count);
            updated.pointer = app.paginator.show(paging);
        },
        MoveBy::Archive => {
            on_first(app, updated, count, ignore_views, move_by);
        }
    }
}

pub fn on_shuffle(app: &mut App, updated: &mut Updated, fix_current: bool) {
    let serial = app.store();
    let app_info = app.app_info();
    app.entries.shuffle(&app_info);

    if fix_current {
        app.restore_or_first(updated, serial);
        updated.image = 1 < app.gui.len();
    } else {
        updated.image = true;
        updated.pointer = true;
    }
    updated.label = true;
}

pub fn on_sort(app: &mut App, updated: &mut Updated, fix_current: bool) {
    let serial = app.store();
    let app_info = app.app_info();

    app.entries.sort(&app_info);

    if fix_current {
        app.restore_or_first(updated, serial);
        updated.image = 1 < app.gui.len();
    } else {
        updated.image = true;
        updated.pointer = true;
    }
}

pub fn on_spawn(app: &mut App) {
    app.states.spawned = true;
    app.operate(Operation::Draw);
}

pub fn on_tell_region(app: &mut App, left: f64, top: f64, right: f64, bottom: f64, button: &Key) {
    let (mx, my) = (left as i32, top as i32);
    for (index, cell) in app.gui.cells(app.states.reverse).enumerate() {
        if app.current_with(index).is_some() {
            let (x1, y1, w, h) = {
                let (cx, cy, cw, ch) = cell.get_top_left();
                if let Some((iw, ih)) = cell.get_image_size() {
                    (cx + (cw - iw) / 2, cy + (ch - ih) / 2, iw, ih)
                } else {
                    continue;
                }
            };
            let (x2, y2) = (x1 + w, y1 + h);
            if x1 <= mx && mx <= x2 && y1 <= my && my <= y2 {
                let (w, h) = (f64!(w) , f64!(h));
                let region = Region::new(
                    f64!(mx - x1) / w,
                    f64!(my - y1) / h,
                    (right - f64!(x1)) / w,
                    (bottom - f64!(y1)) / h);
                let op = Operation::Input(Input::Region(region, button.clone(), index));
                app.tx.send(op).unwrap();
            }
        }
    }
}

pub fn on_timer(app: &mut App, name: String, op: Vec<String>, interval: Duration, repeat: Option<usize>) {
    app.timers.register(name, op, interval, repeat);
}

pub fn on_unclip(app: &mut App, updated: &mut Updated) {
    app.states.drawing.clipping = None;
    updated.image_options = true;
}

pub fn on_undo(app: &mut App, updated: &mut Updated, count: Option<usize>) {
    // `counted` should be evaluated
    #[cfg_attr(feature = "cargo-clippy", allow(or_fun_call))]
    let count = count.unwrap_or(app.counter.pop());

    if let Some((ref entry, _)) = app.current() {
        app.cache.undo_cherenkov(&entry.key, count)
    }
    updated.image_options = true;
}

pub fn on_unmap(app: &mut App, target: &MappingTarget) {
    use app::MappingTarget::*;

    // puts_event!("unmap", "target" => format!("{:?}", target), "operation" => format!("{:?}", operation));
    match *target {
        Unified(ref key_sequence, ref region) =>
            app.mapping.unregister_unified(key_sequence, region),
        Event(ref event_name, ref group) =>
            app.mapping.unregister_event(event_name, group),
        Region(ref button) =>
            app.mapping.unregister_region(button),
    }
}

pub fn on_update_option(app: &mut App, updated: &mut Updated, option_name: &OptionName, updater: &OptionUpdater) {
    use option::OptionValue;
    use operation::option::OptionName::*;
    use operation::option::OptionUpdater::*;
    use operation::option::PreDefinedOptionName::*;
    use size;

    let mut dummy_switch = DummySwtich::new();

    {
        let value: &mut OptionValue = match *option_name {
            PreDefined(ref option_name) => match *option_name {
                AbbrevLength => &mut app.states.abbrev_length,
                AutoPaging => &mut app.states.auto_paging,
                CenterAlignment => &mut app.states.view.center_alignment,
                CurlConnectTimeout => &mut app.states.curl_options.connect_timeout,
                CurlFollowLocation => &mut app.states.curl_options.follow_location,
                CurlLowSpeedLimit => &mut app.states.curl_options.low_speed_limit,
                CurlLowSpeedTime => &mut app.states.curl_options.low_speed_time,
                CurlTimeout => &mut app.states.curl_options.timeout,
                EmptyStatusFormat => &mut app.states.empty_status_format,
                FitTo => &mut app.states.drawing.fit_to,
                HistoryFile => &mut app.states.history_file,
                HorizontalViews => &mut app.states.view.cols,
                LogFile => &mut app.states.log_file,
                MaskOperator => &mut app.states.drawing.mask_operator,
                PathList => &mut app.states.path_list,
                PreFetchEnabled => &mut app.states.pre_fetch.enabled,
                PreFetchLimit => &mut app.states.pre_fetch.limit_of_items,
                PreFetchPageSize => &mut app.states.pre_fetch.page_size,
                Reverse => &mut app.states.reverse,
                Rotation => &mut app.states.drawing.rotation,
                SkipResizeWindow => &mut app.states.skip_resize_window,
                StatusBar => &mut app.states.status_bar,
                StatusFormat => &mut app.states.status_format,
                StdOut => &mut app.states.stdout,
                TitleFormat => &mut app.states.title_format,
                UpdateCacheAccessTime => &mut app.states.update_cache_atime,
                VerticalViews => &mut app.states.view.rows,
                ColorWindowBackground => &mut app.gui.colors.window_background,
                ColorStatusBar => &mut app.gui.colors.status_bar,
                ColorStatusBarBackground => &mut app.gui.colors.status_bar_background,
                ColorError => &mut app.gui.colors.error,
                ColorErrorBackground => &mut app.gui.colors.error_background,
            },
            UserDefined(ref option_name) => {
                if let Some(switch) = app.user_switches.get(option_name) {
                    switch
                } else {
                    dummy_switch.rename(o!(option_name));
                    &mut dummy_switch
                }
            }
        };


        match *updater {
            Increment(_) | Decrement(_) if *option_name == PreDefined(FitTo) => {
                match app.states.drawing.fit_to {
                    size::FitTo::Scale(_) =>
                        (),
                    _ =>
                        value.set(&format!("{}%", (app.current_base_scale.unwrap_or(1.0) * 100.0) as usize)).unwrap(),
                };
            },
            _ => (),
        };

        let result = match *updater {
            Cycle(ref reverse) => value.cycle(*reverse),
            Disable => value.disable(),
            Enable => value.enable(),
            Set(ref arg) => value.set(arg),
            Toggle => value.toggle(),
            Unset => value.unset(),
            SetByCount => value.set_from_count(app.counter.pop_option()),
            Increment(delta) => value.increment(app.counter.pop_option().unwrap_or(delta)),
            Decrement(delta) => value.decrement(app.counter.pop_option().unwrap_or(delta)),
        };

        if let Err(error) = result {
            puts_error!(error, "at" => "update_option", "for" => d!(option_name));
            return;
        }
    }

    updated.image = true;

    if let PreDefined(ref option_name) = *option_name {
        app.update_env_for_option(option_name);
        if option_name.is_for_curl() {
            app.remote_cache.update_curl_options(app.states.curl_options.clone());
        }
        match *option_name {
            AbbrevLength =>
                updated.label = true,
            StatusBar => {
                app.update_label_visibility();
                updated.image_options = true;
            }
            CenterAlignment => {
                app.reset_view();
                updated.image_options = true;
            }
            FitTo | Rotation =>
                updated.image_options = true,
            PreFetchLimit =>
                app.cache.update_limit(app.states.pre_fetch.limit_of_items),
            ColorWindowBackground | ColorStatusBar | ColorStatusBarBackground =>
                app.gui.update_colors(),
            VerticalViews | HorizontalViews =>
                on_update_views(app, updated),
            UpdateCacheAccessTime =>
                app.remote_cache.do_update_atime = app.states.update_cache_atime,
            _ => ()
        }
    }
}

pub fn on_user(_: &mut App, data: &[(String, String)]) {
    let mut pairs = vec![(o!("event"), o!("user"))];
    pairs.extend_from_slice(data);
    logger::puts(&pairs);
}

pub fn on_views(app: &mut App, updated: &mut Updated, cols: Option<usize>, rows: Option<usize>) {
    if let Some(cols) = cols {
        app.states.view.cols = cols
    }
    if let Some(rows) = rows {
        app.states.view.rows = rows
    }
    on_update_views(app, updated);
}

pub fn on_views_fellow(app: &mut App, updated: &mut Updated, for_rows: bool) {
    let count = app.counter.pop();
    if for_rows {
        app.states.view.rows = count;
    } else {
        app.states.view.cols = count;
    };
    on_update_views(app, updated);
}

pub fn on_when(app: &mut App, filter: FilterExpr, unless: bool, op: &[String]) {
    let app_info = app.app_info();
    if_let_some!((_, index, _) = app.current_non_fly_leave(), ());
    if_let_some!(r = app.entries.validate_nth(index, filter, &app_info), ());

    if r ^ unless {
        match Operation::parse_from_vec(op) {
            Ok(op) =>
                app.operate(op),
            Err(err) =>
                puts_error!(err, "at" => "input")
        }
    }
}

pub fn on_window_resized(app: &mut App, updated: &mut Updated) {
    updated.image_options = true;
    // Ignore followed PreFetch
    app.pre_fetch_serial += 1;
}

pub fn on_with_message(app: &mut App, updated: &mut Updated, message: Option<String>, op: Operation) {
    updated.message = true;
    app.update_message(message);
    app.tx.send(Operation::UpdateUI).unwrap();
    app.tx.send(op).unwrap();
}

pub fn on_write(app: &mut App, path: &PathBuf, index: &Option<usize>) {
    let count = index.unwrap_or_else(|| app.counter.pop()) - 1;
    if let Err(error) = app.gui.save(path, count) {
        puts_error!(error, "at" => "save")
    }
}

fn on_update_views(app: &mut App, updated: &mut Updated) {
    updated.image_options = true;
    let serial = app.store();
    app.reset_view();
    app.restore_or_first(updated, serial);
}

fn push_buffered(app: &mut App, updated: &mut Updated, ops: Vec<QueuedOperation>) {
    use operation::QueuedOperation::*;

    let before_len = app.entries.len();
    let app_info = app.app_info();

    for op in ops {
        match op {
            PushImage(path, meta, force, expand_level, url) =>
                app.entries.push_image(&app_info, &path, meta, force, expand_level, url),
            PushDirectory(path, meta, force) =>
                app.entries.push_directory(&app_info, &path, meta, force),
            PushArchive(archive_path, meta, force, url) =>
                on_push_archive(app, &archive_path, meta, force, url),
            PushArchiveEntry(archive_path, entry, meta, force, url) =>
                app.entries.push_archive_entry(&app_info, &archive_path, &entry, meta, force, url),
            PushPdf(pdf_path, meta, force, url) =>
                on_push_pdf(app, updated, pdf_path, meta, force, url),
            PushPdfEntries(pdf_path, pages, meta, force, url) => {
                let pdf_path = Arc::new(pdf_path.clone());
                for index in 0 .. pages {
                    app.entries.push_pdf_entry(&app_info, pdf_path.clone(), index, meta.clone(), force, url.clone());
                }
            }
        }
        updated.label = true;
    }

    app.update_paginator_condition();
    app.remote_cache.update_sorting_buffer_len();

    if before_len == 0 && 0 < app.entries.len() {
        updated.pointer |= app.paginator.reset_level()
    }

    app.do_go(updated);
}

fn extract_region_from_context(context: Option<OperationContext>) -> Option<(Region, usize)> {
    if let Some(input) = context.map(|it| it.input) {
        if let Input::Region(ref region, _, cell_index) = input {
            return Some((*region, cell_index));
        }
    }
    None
}

fn set_count_env(app: &mut App) {
    let count = app.counter.pop();
    env::set_var(format!("{}COUNT", VARIABLE_PREFIX), s!(count));
}

fn is_url(path: &str) -> bool {
    if_let_some!(index = path.find("://"), false);
    index < 10
}
