
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
use cherenkov::Filler;
use color::Color;
use config::DEFAULT_CONFIG;
use editor;
use entry::filter::expression::Expr as FilterExpr;
use entry::{self, Meta, SearchKey, Entry,EntryContent};
use expandable::{Expandable, expand_all};
use filer;
use fragile_input::new_fragile_input;
use gui::Direction;
use mapping;
use operation::{self, Operation, OperationContext, MappingTarget, MoveBy, OptionName, OptionUpdater};
use option::user::DummySwtich;
use output;
use poppler::{PopplerDocument, self};
use script;
use session::{Session, write_sessions};
use shell;
use shell_filter;
use state;
use utils::path_to_str;

use app::*;



pub fn on_cherenkov(app: &mut App, updated: &mut Updated, parameter: &operation::CherenkovParameter, context: Option<OperationContext>) {
    use cherenkov::{Che, CheNova};

    if let Some(Input::MouseButton((mx, my), _)) = context.map(|it| it.input) {
        let cell_size = app.gui.get_cell_size(&app.states.view, app.states.status_bar);

        for (index, cell) in app.gui.cells(app.states.reverse).enumerate() {
            if let Some(entry) = app.entries.current_with(&app.pointer, index).map(|(entry,_)| entry) {
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
                    let center = (
                        parameter.x.unwrap_or_else(|| mx - x1) as f64 / w as f64,
                        parameter.y.unwrap_or_else(|| my - y1) as f64 / h as f64);
                    app.cache.cherenkov(
                        &entry,
                        &cell_size,
                        &Che::Nova(CheNova {
                            center: center,
                            n_spokes: parameter.n_spokes,
                            radius: parameter.radius,
                            random_hue: parameter.random_hue,
                            color: parameter.color,
                        }),
                        &app.states.drawing);
                    updated.image = true;
                }
            }
        }
    }
}

pub fn on_clear(app: &mut App, updated: &mut Updated) {
    app.entries.clear(&mut app.pointer);
    app.cache.clear();
    updated.image = true;
}

pub fn on_clip(app: &mut App, updated: &mut Updated, inner: Region, context: Option<OperationContext>) {
    let inner = extract_region_from_context(context).map(|it| it.0).unwrap_or(inner);
    let current = app.states.drawing.clipping.unwrap_or_default();
    app.states.drawing.clipping = Some(current + inner);
    updated.image_options = true;
}

pub fn on_editor(app: &mut App, editor_command: Option<Expandable>, files: &[Expandable], sessions: &[Session]) {
    let tx = app.tx.clone();
    let source = with_ouput_string!(out, {
        for file in files {
            if let Err(err) = File::open(file.to_path_buf()).and_then(|mut file| file.read_to_string(out)) {
                puts_error!("at" => o!("on_load"), "reason" => s!(err));
            }
        }
        write_sessions(app, sessions, out);
    });
    spawn(move || editor::start_edit(&tx, editor_command.map(|it| it.to_string()), &source));
}

pub fn on_expand(app: &mut App, updated: &mut Updated, recursive: bool, base: Option<PathBuf>) {
    let count = app.pointer.counted();
    if recursive {
        app.entries.expand(&mut app.pointer, base, 1, count as u8);
    } else {
        app.entries.expand(&mut app.pointer, base, count as u8, count as u8- 1);
    }
    updated.label = true;
}

pub fn on_define_switch(app: &mut App, name: String, values: Vec<Vec<String>>) {
    if let Err(error) = app.user_switches.register(name, values) {
        puts_error!("at" => "on_define_switch", "reason" => error);
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
pub fn on_fill(app: &mut App, updated: &mut Updated, filler: Filler, region: Option<Region>, color: Color, mask: bool, cell_index: usize, context: Option<OperationContext>) {
    use cherenkov::Che;

    let (region, cell_index) = extract_region_from_context(context)
        .or_else(|| region.map(|it| (it, cell_index)))
        .unwrap_or_else(|| (Region::full(), cell_index));

    if let Some(entry) = app.entries.current_with(&app.pointer, cell_index).map(|(entry,_)| entry) {
        let cell_size = app.gui.get_cell_size(&app.states.view, app.states.status_bar);
        app.cache.cherenkov(
            &entry,
            &cell_size,
            &Che::Fill(filler, region, color, mask),
            &app.states.drawing);
        updated.image = true;
    }
}

pub fn on_filter(app: &mut App, updated: &mut Updated, expr: Option<FilterExpr>) {
    app.states.last_filter = expr.clone();
    if let Some(expr) = expr {
        app.entries.update_filter(&mut app.pointer, Some(Box::new(move |ref mut entry| expr.evaluate(entry))));
    } else {
        app.entries.update_filter(&mut app.pointer, None);
    }
    updated.pointer = true;
    updated.image = true;
}

pub fn on_first(app: &mut App, updated: &mut Updated, len: usize, count: Option<usize>, ignore_views: bool, move_by: MoveBy) {
    match move_by {
        MoveBy::Page =>
            updated.pointer = app.pointer.with_count(count).first(len, !ignore_views),
        MoveBy::Archive => {
            let count = app.pointer.with_count(count).counted();
            if let Some(first) = app.entries.find_nth_archive(count, false) {
                app.pointer.current = Some(first);
                updated.pointer = true;
            }
        }
    }
}

pub fn on_fragile(app: &mut App, path: &Expandable) {
    new_fragile_input(app.tx.clone(), &path.to_path_buf());
}

pub fn on_initialized(app: &mut App) {
    app.states.initialized = true;
    app.gui.update_colors();
    app.tx.send(Operation::Draw).unwrap();
    puts_event!("initialized");
    fire_event(app, "initialize");
}

pub fn on_input(app: &mut App, input: &Input) {
    let (width, height) = app.gui.window.get_size();
    let operations = app.mapping.matched(input, width, height);

    if operations.is_empty() {
        puts_event!("input", "type" => input.type_name(), "name" => input.text());
        return;
    }

    for op in operations {
        match Operation::parse_from_vec(&op) {
            Ok(op) =>
                app.operate(Operation::Context(OperationContext { input: input.clone(), cell_index: None }, Box::new(op))),
            Err(err) =>
                puts_error!("at" => "input", "reason" => err)
        }
    }
}

pub fn on_kill_timer(app: &mut App, name: &str) {
    app.timers.unregister(name);
}

pub fn on_last(app: &mut App, updated: &mut Updated, len: usize, count: Option<usize>, ignore_views: bool, move_by: MoveBy) {
    match move_by {
        MoveBy::Page =>
            updated.pointer = app.pointer.with_count(count).last(len, !ignore_views),
        MoveBy::Archive => {
            let count = app.pointer.with_count(count).counted();
            if let Some(nth) = app.entries.find_nth_archive(count, true) {
                app.pointer.current = Some(nth);
                updated.pointer = true;
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

pub fn on_load(app: &mut App, file: &Path) {
    script::load_from_file(&app.tx, file);
}

pub fn on_load_default(app: &mut App) {
    script::load(&app.tx, DEFAULT_CONFIG);
}

pub fn on_map(app: &mut App, target: MappingTarget, operation: Vec<String>) {
    use app::MappingTarget::*;

    // puts_event!("map", "target" => format!("{:?}", target), "operation" => format!("{:?}", operation));
    match target {
        Key(key_sequence) =>
            app.mapping.register_key(key_sequence, operation),
        Mouse(button, area) =>
            app.mapping.register_mouse(button, area, operation),
        Event(event_name, id) =>
            app.mapping.register_event(event_name, id, operation),
        Region(button) =>
            app.mapping.register_region(button, operation),
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
pub fn on_move_again(app: &mut App, updated: &mut Updated, len: usize, to_end: &mut bool, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool) {
    if app.states.last_direction == state::Direction::Forward {
        on_next(app, updated, len, count, ignore_views, move_by, wrap)
    } else {
        on_previous(app, updated, len, to_end, count, ignore_views, move_by, wrap)
    }
}

pub fn on_move_entry(app: &mut App, updated: &mut Updated, from: &entry::Position, to: &entry::Position) {
    updated.image = app.entries.move_entry(&app.pointer, from, to);
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

pub fn on_next(app: &mut App, updated: &mut Updated, len: usize, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool) {
    app.states.last_direction = state::Direction::Forward;
    match move_by {
        MoveBy::Page =>
            updated.pointer = app.pointer.with_count(count).next(len, !ignore_views, wrap),
        MoveBy::Archive => {
            let count = app.pointer.with_count(count).counted();
            if let Some(next) = app.entries.find_next_archive(&app.pointer, count) {
                app.pointer.current = Some(next);
                updated.pointer = true;
            }
        }
    }
}

pub fn on_operate_file(app: &mut App, file_operation: &filer::FileOperation) {
    use entry::EntryContent::*;

    if let Some((entry, _)) = app.entries.current(&app.pointer) {
        let result = match entry.content {
            File(ref path) | Http(ref path, _) => file_operation.execute(path),
            Archive(ref path , ref entry) => file_operation.execute_with_buffer(&entry.content.clone(), path),
            _ => not_implemented!(),
        };
        let text = format!("{:?}", file_operation);
        match result {
            Ok(_) => puts_event!("operate_file", "status" => "ok", "operation" => text),
            Err(err) => puts_event!("operate_file", "status" => "fail", "reason" => err, "operation" => text),
        }
    }
}

pub fn on_pdf_index(app: &App, async: bool, read_operations: bool, command_line: &[Expandable], fmt: &poppler::index::Format, tx: Sender<Operation>) {
    if_let_some!((entry, _) = app.entries.current(&app.pointer), ());
    if let EntryContent::Pdf(path, _) = entry.content {
        let mut stdin = o!("");
        PopplerDocument::new_from_file(&*path).index().write(fmt, &mut stdin);
        shell::call(async, &expand_all(command_line), Some(stdin), option!(read_operations, tx));
    } else {
        puts_error!("at" => "on_pdf_index", "reason" => "current entry is not PDF");
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
pub fn on_previous(app: &mut App, updated: &mut Updated, len: usize, to_end: &mut bool, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool) {
    app.states.last_direction = state::Direction::Backward;
    match move_by {
        MoveBy::Page => {
            updated.pointer = app.pointer.with_count(count).previous(len, !ignore_views, wrap);
            *to_end = count.is_none() && !ignore_views;
        }
        MoveBy::Archive => {
            let count = app.pointer.with_count(count).counted();
            if let Some(previous) = app.entries.find_previous_archive(&app.pointer, count) {
                app.pointer.current = Some(previous);
                updated.pointer = true;
            }
        }
    }
}

pub fn on_print_entries(app: &App) {
    use std::io::{Write, stderr};
    for entry in app.entries.to_displays() {
        writeln!(&mut stderr(), "{}", entry).unwrap();
    }
}

pub fn on_pull(app: &mut App, updated: &mut Updated) {
    let buffered = app.sorting_buffer.pull_all();
    push_buffered(app, updated, buffered);
}

pub fn on_push(app: &mut App, updated: &mut Updated, path: String, meta: Option<Meta>, force: bool) {
    if path.starts_with("http://") || path.starts_with("https://") {
        app.tx.send(Operation::PushURL(path, meta, force)).unwrap();
        return;
    }

    on_push_path(app, updated, &Path::new(&path).to_path_buf(), meta, force)
}

pub fn on_push_path(app: &mut App, updated: &mut Updated, path: &PathBuf, meta: Option<Meta>, force: bool) {
    if let Ok(path) = path.canonicalize() {
        if let Some(ext) = path.extension() {
            match &*ext.to_str().unwrap().to_lowercase() {
                "zip" | "rar" | "tar.gz" | "lzh" | "lha" =>
                    return archive::fetch_entries(&path, &app.encodings, app.tx.clone(), app.sorting_buffer.clone(), force),
                "pdf" =>
                    return on_push_pdf(app, updated, path.to_path_buf(), meta, force),
                _ => ()
            }
        }
    }

    if path.is_dir() {
        on_push_directory(app, updated, path.clone(), meta, force)
    } else {
        on_push_image(app, updated, path.clone(), meta, force, None)
    }
}

pub fn on_push_directory(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool) {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushDirectory(file, meta, force));
    push_buffered(app, updated, buffered);
}

pub fn on_push_image(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool, expand_level: Option<u8>) {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushImage(file, meta, force, expand_level));
    push_buffered(app, updated, buffered);
}

pub fn on_push_pdf(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool) {
    let document = PopplerDocument::new_from_file(&file);
    let n_pages = document.n_pages();

    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushPdfEntries(file, n_pages, meta, force));
    push_buffered(app, updated, buffered);
}

pub fn on_push_sibling(app: &mut App, updated: &mut Updated, next: bool, meta: Option<Meta>, force: bool, show: bool) {
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

    let found = app.entries.current(&app.pointer).and_then(|(entry, _)| {
        match entry.content {
            File(ref path) | Http(ref path, _) =>
                find_sibling(path, next),
                Archive(ref path, _) | Pdf(ref path, _) =>
                    find_sibling(&*path, next),
        }
    });

    if let Some(found) = found {
        if show {
            on_show(app, updated, &SearchKey { path: o!(path_to_str(&found)), index: None});
        }
        on_push_path(app, updated, &found, meta, force);
    }
}

pub fn on_push_url(app: &mut App, updated: &mut Updated, url: String, meta: Option<Meta>, force: bool) {
    let buffered = app.http_cache.fetch(url, meta, force);
    push_buffered(app, updated, buffered);
}

pub fn on_quit(app: &mut App) {
    fire_event(app, "quit");
    termination::execute();
}

pub fn on_random(app: &mut App, updated: &mut Updated, len: usize) {
    if len > 0 {
        app.pointer.current = Some(RandRange::new(0, len).ind_sample(&mut app.rng));
        updated.image = true;
    }
}

pub fn on_reset_image(app: &mut App, updated: &mut Updated) {
    if let Some(entry) = app.entries.current_entry(&app.pointer) {
        app.cache.uncherenkov(&entry);
        updated.image_options = true;
    }
}

pub fn on_save(app: &mut App, path: &Option<PathBuf>, sessions: &[Session]) {
    let default = app_path::config_file(Some(app_path::DEFAULT_SESSION_FILENAME));
    let path = path.as_ref().unwrap_or(&default);

    let result = File::create(path).map(|mut file| {
        file.write_all(with_ouput_string!(out, write_sessions(app, sessions, out)).as_str().as_bytes())
    });

    if let Err(err) = result {
        puts_error!("at" => "save_session", "reason" => s!(err))
    }
}

pub fn on_search_text(app: &mut App, updated: &mut Updated, text: Option<String>, backward: bool) {
    if let Some(text) = text {
        app.search_text = Some(text);
    }

    if_let_some!(text = app.search_text.clone(), ());

    let seq: Vec<(usize, &Rc<Entry>)> = if backward {
        let skip = app.pointer.current.map(|index| app.entries.len() - index).unwrap_or(0);
        app.entries.iter().enumerate().rev().skip(skip).collect()
    } else {
        let skip = app.pointer.current.unwrap_or(0) + 1;
        app.entries.iter().enumerate().skip(skip).collect()
    };

    let mut previous: Option<(Rc<PopplerDocument>, PathBuf)> = None;

    for (index, entry) in seq {
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
            if page.find_text(&text) {
                app.pointer.current = Some(index);
                updated.pointer = true;
                return;
            }
        }
    }
}

pub fn on_set_env(_: &mut App, name: &str, value: &Option<String>) {
    if let Some(ref value) = *value {
        env::set_var(name, value);
    } else {
        env::remove_var(name);
    }
}

pub fn on_scroll(app: &mut App, direction: &Direction, operation: &[String], scroll_size: f64) {
    let save = app.pointer.save();
    if !app.gui.scroll_views(direction, scroll_size, app.pointer.counted()) && !operation.is_empty() {
        match Operation::parse_from_vec(operation) {
            Ok(op) => {
                app.pointer.restore(&save);
                app.operate(op);
            },
            Err(err) => puts_error!("at" => "scroll", "reason" => err),
        }
    }
}

pub fn on_shell(app: &App, async: bool, read_operations: bool, command_line: &[Expandable], tx: Sender<Operation>, sessions: &[Session]) {
    let stdin = if !sessions.is_empty() {
        Some(with_ouput_string!(out, write_sessions(app, sessions, out)))
    } else {
        None
    };

    shell::call(async, &expand_all(command_line), stdin, option!(read_operations, tx));
}

pub fn on_shell_filter(app: &App, command_line: &[Expandable]) {
    shell_filter::start(expand_all(command_line), app.tx.clone());
}

pub fn on_show(app: &mut App, updated: &mut Updated, key: &SearchKey) {
    let index = app.entries.search(key);
    if let Some(index) = index {
        app.pointer.current = Some(index);
        updated.pointer = true;
    } else {
        app.states.show = Some(key.clone());
    }
}

pub fn on_shuffle(app: &mut App, updated: &mut Updated, fix_current: bool) {
    app.entries.shuffle(&mut app.pointer, fix_current);
    if !fix_current {
        updated.image = true;
    }
    updated.label = true;
}

pub fn on_sort(app: &mut App, updated: &mut Updated) {
    app.entries.sort(&mut app.pointer);
    updated.image = true;
}

pub fn on_tell_region(app: &mut App, left: f64, top: f64, right: f64, bottom: f64, button: u32) {
    let (mx, my) = (left as i32, top as i32);
    for (index, cell) in app.gui.cells(app.states.reverse).enumerate() {
        if app.entries.current_with(&app.pointer, index).is_some() {
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
                let (w, h) = (w as f64, h as f64);
                let region = Region::new(
                    (mx - x1) as f64 / w,
                    (my - y1) as f64 / h,
                    (right - x1 as f64) / w,
                    (bottom - y1 as f64) / h);
                let op = Operation::Input(Input::Region(region, button, index));
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
    let count = count.unwrap_or(app.pointer.counted());

    if let Some((ref entry, _)) = app.entries.current(&app.pointer) {
        app.cache.undo_cherenkov(entry, count)
    }
    updated.image_options = true;
}

pub fn on_update_option(app: &mut App, updated: &mut Updated, option_name: &OptionName, updater: &OptionUpdater) {
    use option::OptionValue;
    use operation::OptionName::*;
    use operation::PreDefinedOptionName::*;
    use operation::OptionUpdater::*;

    let mut dummy_switch = DummySwtich::new();

    {
        let value: &mut OptionValue = match *option_name {
            PreDefined(ref option_name) => match *option_name {
                AutoPaging => &mut app.states.auto_paging,
                CenterAlignment => &mut app.states.view.center_alignment,
                FitTo => &mut app.states.drawing.fit_to,
                Reverse => &mut app.states.reverse,
                Scaling => &mut app.states.drawing.scaling,
                StatusBar => &mut app.states.status_bar,
                StatusFormat => &mut app.states.status_format,
                TitleFormat => &mut app.states.title_format,
                PreFetchEnabled => &mut app.states.pre_fetch.enabled,
                PreFetchLimit => &mut app.states.pre_fetch.limit_of_items,
                PreFetchPageSize => &mut app.states.pre_fetch.page_size,
                HorizontalViews => &mut app.states.view.cols,
                VerticalViews => &mut app.states.view.rows,
                MaskOperator => &mut app.states.drawing.mask_operator,
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

        let result = match *updater {
            Cycle(ref reverse) => value.cycle(*reverse),
            Disable => value.disable(),
            Enable => value.enable(),
            Set(ref arg) => value.set(arg),
            Toggle => value.toggle(),
            Unset => value.unset(),
        };

        if let Err(error) = result {
            puts_error!("at" => "update_option", "reason" => error);
            return;
        }
    }

    updated.image = true;

    if let PreDefined(ref option_name) = *option_name {
        app.update_env_for_option(option_name);
        match *option_name {
            StatusBar => {
                app.update_label_visibility();
                updated.image_options = true;
            }
            CenterAlignment => {
                app.reset_view();
                updated.image_options = true;
            }
            FitTo =>
                updated.image_options = true,
            PreFetchLimit =>
                app.cache.update_limit(app.states.pre_fetch.limit_of_items),
            ColorWindowBackground | ColorStatusBar | ColorStatusBarBackground =>
                app.gui.update_colors(),
            VerticalViews | HorizontalViews =>
                on_update_views(app, updated),
            _ => ()
        }
    }
}

pub fn on_user(_: &mut App, data: &[(String, String)]) {
    let mut pairs = vec![(o!("event"), o!("user"))];
    pairs.extend_from_slice(data);
    output::puts(&pairs);
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
    let count = app.pointer.counted();
    if for_rows {
        app.states.view.rows = count;
    } else {
        app.states.view.cols = count;
    };
    on_update_views(app, updated);
}

pub fn on_window_resized(app: &mut App, updated: &mut Updated) {
    updated.image_options = true;
    // Ignore followed PreFetch
    app.pre_fetch_serial += 1;
    fire_event(app, "resize-window");
}

pub fn on_write(app: &mut App, path: &PathBuf, index: &Option<usize>) {
    let count = index.unwrap_or_else(|| app.pointer.counted()) - 1;
    if let Err(error) = app.gui.save(path, count) {
        puts_error!("at" => "save", "reason" => error)
    }
}

pub fn fire_event(app: &mut App, event_name: &str) {
    app.operate(Operation::Input(mapping::Input::Event(o!(event_name))));
}

fn on_update_views(app: &mut App, updated: &mut Updated) {
    updated.image_options = true;
    app.reset_view();
    app.pointer.set_multiplier(app.gui.len());
}

fn push_buffered(app: &mut App, updated: &mut Updated, ops: Vec<QueuedOperation>) {
    use operation::QueuedOperation::*;

    for op in ops {
        match op {
            PushImage(path, meta, force, expand_level) =>
                updated.pointer = app.entries.push_image(&mut app.pointer, &path, meta, force, expand_level),
            PushDirectory(path, meta, force) =>
                updated.pointer = app.entries.push_directory(&mut app.pointer, &path, meta, force),
            PushHttpCache(file, url, meta, force) =>
                updated.pointer |= app.entries.push_http_cache(&mut app.pointer, &file, url, meta, force),
            PushArchiveEntry(ref archive_path, ref entry, force) =>
                updated.pointer |= app.entries.push_archive_entry(&mut app.pointer, archive_path, entry, force),
            PushPdfEntries(pdf_path, pages, meta, force) => {
                let pdf_path = Arc::new(pdf_path.clone());
                for index in 0 .. pages {
                    updated.pointer |= app.entries.push_pdf_entry(&mut app.pointer, pdf_path.clone(), index, meta.clone(), force)
                }
            }
        }
        updated.label = true;
    }
    app.do_show(updated);
}

fn extract_region_from_context(context: Option<OperationContext>) -> Option<(Region, usize)> {
    if let Some(input) = context.map(|it| it.input) {
        if let Input::Region(ref region, _, cell_index) = input {
            return Some((*region, cell_index));
        }
    }
    None
}
