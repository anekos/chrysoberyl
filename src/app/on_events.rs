
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;

use gtk::prelude::*;
use rand::distributions::{IndependentSample, Range as RandRange};

use app_path;
use archive;
use editor;
use entry::{self, Meta, SearchKey};
use filer;
use filter;
use fragile_input::new_fragile_input;
use gui::Direction;
use operation::{self, Operation, OperationContext, MappingTarget, MoveBy, OptionName, OptionUpdater, StdinSource};
use option::user::DummySwtich;
use output;
use poppler::PopplerDocument;
use script;
use shell;
use state::RegionFunction;
use utils::path_to_str;

use app::*;



pub fn on_cherenkov(app: &mut App, updated: &mut Updated, parameter: &operation::CherenkovParameter, context: &Option<OperationContext>) {
    use cherenkov::{Che, CheNova};

    if let Some(OperationContext::Input(Input::MouseButton((mx, my), _))) = *context {
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

pub fn on_cherenkov_clear(app: &mut App, updated: &mut Updated) {
    if let Some(entry) = app.entries.current_entry(&app.pointer) {
        app.cache.uncherenkov(&entry);
        updated.image_options = true;
    }
}

pub fn on_clear(app: &mut App, updated: &mut Updated) {
    app.entries.clear(&mut app.pointer);
    app.cache.clear();
    updated.image = true;
}

pub fn on_clip(app: &mut App, updated: &mut Updated, inner: Region) {
    let current = app.states.drawing.clipping.unwrap_or_default();
    app.states.drawing.clipping = Some(current + inner);
    updated.image_options = true;
}

pub fn on_editor(app: &mut App, editor_command: Option<String>, script_sources: Vec<script::ScriptSource>) {
    let tx = app.tx.clone();
    spawn(move || editor::start_edit(&tx, editor_command, script_sources));
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

pub fn on_fill(app: &mut App, updated: &mut Updated, region: Region, cell_index: usize) {
    use cherenkov::Che;

    if let Some(entry) = app.entries.current_with(&app.pointer, cell_index).map(|(entry,_)| entry) {
        let cell_size = app.gui.get_cell_size(&app.states.view, app.states.status_bar);
        app.cache.cherenkov(
            &entry,
            &cell_size,
            &Che::Fill(region, app.states.fill_color),
            &app.states.drawing);
        updated.image = true;
    }
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

pub fn on_fragile(app: &mut App, path: &PathBuf) {
    new_fragile_input(app.tx.clone(), path_to_str(path));
}

pub fn on_filter(app: &App, command_line: Vec<String>) {
    filter::start(command_line, app.tx.clone());
}

pub fn on_initialized(app: &mut App) {
    app.states.initialized = true;
    app.gui.update_colors();
    app.tx.send(Operation::Draw).unwrap();
    puts_event!("initialized");
}

pub fn on_input(app: &mut App, input: &Input) {
    let (width, height) = app.gui.window.get_size();
    if let Some(op) = app.mapping.matched(input, width, height) {
        match op {
            Ok(op) =>
                app.operate(Operation::Context(OperationContext::Input(input.clone()), Box::new(op))),
            Err(err) =>
                puts_error!("at" => "input", "reason" => err)
        }
    } else {
        puts_event!("input", "type" => input.type_name(), "name" => input.text());
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
        updated.image = true;
        *to_end = new_to_end;
    }
}

pub fn on_load(app: &mut App, script_source: &script::ScriptSource) {
    script::load(&app.tx, script_source);
}

pub fn on_map(app: &mut App, target: &MappingTarget, operation: Vec<String>) {
    use app::MappingTarget::*;

    // puts_event!("map", "target" => format!("{:?}", target), "operation" => format!("{:?}", operation));
    match *target {
        Key(ref key_sequence) =>
            app.mapping.register_key(key_sequence.clone(), operation),
        Mouse(ref button, ref area) =>
            app.mapping.register_mouse(*button, area.clone(), operation)
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

    if let Ok(path) = Path::new(&path).canonicalize() {
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

    app.operate(Operation::PushPath(Path::new(&path).to_path_buf(), meta, force));
}

pub fn on_push_path(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool) {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushPath(file, meta, force));
    push_buffered(app, updated, buffered);
}

pub fn on_push_pdf(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool) {
    let document = PopplerDocument::new_from_file(&file);
    let n_pages = document.n_pages();

    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushPdfEntries(file, n_pages, meta, force));
    push_buffered(app, updated, buffered);
}

pub fn on_push_url(app: &mut App, updated: &mut Updated, url: String, meta: Option<Meta>, force: bool) {
    let buffered = app.http_cache.fetch(url, meta, force);
    push_buffered(app, updated, buffered);
}

pub fn on_random(app: &mut App, updated: &mut Updated, len: usize) {
    if len > 0 {
        app.pointer.current = Some(RandRange::new(0, len).ind_sample(&mut app.rng));
        updated.image = true;
    }
}

pub fn on_save(app: &mut App, path: &Option<PathBuf>, sources: &[StdinSource]) {
    let default = app_path::config_file(Some(app_path::DEFAULT_SESSION_FILENAME));
    let path = path.as_ref().unwrap_or(&default);

    let result = File::create(path).map(|mut file| {
        file.write_all(sources_to_string(app, sources).as_bytes())
    });

    if let Err(err) = result {
        puts_error!("at" => "save_session", "reason" => s!(err))
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

pub fn on_shell(app: &App, async: bool, read_operations: bool, command_line: &[String], tx: Sender<Operation>, stdin_sources: &[StdinSource]) {
    let stdin = if !stdin_sources.is_empty() {
        Some(sources_to_string(app, stdin_sources))
    } else {
        None
    };

    shell::call(async, command_line, stdin, option!(read_operations, tx));
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

pub fn on_tell_region(app: &mut App, region: &Region) {
    let (mx, my) = (region.left as i32, region.top as i32);
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
                    (region.right - x1 as f64) / w,
                    (region.bottom - y1 as f64) / h);
                let op = match app.states.region_function {
                    RegionFunction::Clip =>
                        Operation::Clip(region),
                    RegionFunction::Fill =>
                        Operation::Fill(region, index),
                };
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

pub fn on_update_option(app: &mut App, updated: &mut Updated, option_name: &OptionName, updater: OptionUpdater) {
    use option::OptionValue;
    use operation::OptionName::*;
    use operation::OptionUpdater::*;

    let mut dummy_switch = DummySwtich::new();

    {
        let value: &mut OptionValue = match *option_name {
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
            RegionFunction => &mut app.states.region_function,
            ColorWindowBackground => &mut app.gui.colors.window_background,
            ColorStatusBar => &mut app.gui.colors.status_bar,
            ColorStatusBarBackground => &mut app.gui.colors.status_bar_background,
            ColorError => &mut app.gui.colors.error,
            ColorErrorBackground => &mut app.gui.colors.error_background,
            ColorFill => &mut app.states.fill_color,
            User(ref name) => {
                if let Some(switch) = app.user_switches.get(name) {
                    switch
                } else {
                    dummy_switch.rename(o!(name));
                    &mut dummy_switch
                }
            }
        };

        let result = match updater {
            Cycle(reverse) => value.cycle(reverse),
            Disable => value.disable(),
            Enable => value.enable(),
            Set(arg) => value.set(&arg),
            Toggle => value.toggle(),
            Unset => value.unset(),
        };

        if let Err(error) = result {
            puts_error!("at" => "update_option", "reason" => error);
            return;
        }
    }

    updated.image = true;

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

pub fn on_user(_: &App, data: &[(String, String)]) {
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
}

pub fn on_write(app: &mut App, path: &PathBuf, index: &Option<usize>) {
    let count = index.unwrap_or_else(|| app.pointer.counted()) - 1;
    if let Err(error) = app.gui.save(path, count) {
        puts_error!("at" => "save", "reason" => error)
    }
}


fn on_update_views(app: &mut App, updated: &mut Updated) {
    updated.image_options = true;
    app.reset_view();
    app.pointer.set_multiplier(app.gui.len());
}

fn sources_to_string(app: &App, sources: &[StdinSource]) -> String {
    use stringer::*;
    use operation::StdinSource::*;

    let mut result = o!("");
    for source in sources {
        match *source {
            Options => write_options(&app.states, &app.gui, &mut result),
            Entries => write_entries(&app.entries, &mut result),
            Paths => write_paths(&app.entries, &mut result),
            Position => write_position(&app.entries, &app.pointer, &mut result),
            Mappings => write_mappings(&app.mapping, &mut result),
            Session => {
                write_options(&app.states, &app.gui, &mut result);
                write_entries(&app.entries, &mut result);
                write_position(&app.entries, &app.pointer, &mut result);
                write_mappings(&app.mapping, &mut result);
            }
        }
    }
    result
}

fn push_buffered(app: &mut App, updated: &mut Updated, ops: Vec<QueuedOperation>) {
    use operation::QueuedOperation::*;

    for op in ops {
        match op {
            PushPath(path, meta, force) =>
                updated.pointer = app.entries.push_path(&mut app.pointer, &path, meta, force),
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
