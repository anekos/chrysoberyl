
use std::env;
use std::path::{Path, PathBuf};
use std::thread::spawn;

use gtk::prelude::*;
use rand::distributions::{IndependentSample, Range as RandRange};

use archive::{self, ArchiveEntry};
use config;
use editor;
use entry::{MetaSlice, new_meta, SearchKey};
use filer;
use fragile_input::new_fragile_input;
use gui::Direction;
use operation::{self, Operation, OperationContext, MappingTarget, MoveBy, OptionName, OptionUpdater};
use output;
use utils::path_to_str;

use app::*;



pub fn on_cherenkov(app: &mut App, updated: &mut Updated, parameter: &operation::CherenkovParameter, context: Option<&OperationContext>) {
    use cherenkov::Che;

    if let Some(OperationContext::Input(Input::MouseButton((mx, my), _))) = context.cloned() {
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
                        &Che {
                            center: center,
                            n_spokes: parameter.n_spokes,
                            radius: parameter.radius,
                            random_hue: parameter.random_hue,
                            color: parameter.color,
                        },
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
    updated.image = true;
}

pub fn on_clip(app: &mut App, updated: &mut Updated, region: &Region) {
    let (mx, my) = (region.left as i32, region.top as i32);
    let current = app.states.drawing.clipping.unwrap_or_default();
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
                let inner = Region::new(
                    (mx - x1) as f64 / w,
                    (my - y1) as f64 / h,
                    (region.right - x1 as f64) / w,
                    (region.bottom - y1 as f64) / h);
                app.states.drawing.clipping = Some(current + inner);
                updated.image_options = true;
            }
        }
    }
}

pub fn on_editor(app: &mut App, editor_command: Option<String>, config_sources: Vec<config::ConfigSource>) {
    let tx = app.tx.clone();
    spawn(move || editor::start_edit(&tx, editor_command, config_sources));
}

pub fn on_expand(app: &mut App, updated: &mut Updated, recursive: bool, base: &Option<PathBuf>) {
    let count = app.pointer.counted();
    if recursive {
        app.entries.expand(&mut app.pointer, base.clone(), 1, count as u8);
    } else {
        app.entries.expand(&mut app.pointer, base.clone(), count as u8, count as u8- 1);
    }
    updated.label = true;
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
                app.operate(&Operation::Context(OperationContext::Input(input.clone()), Box::new(op))),
            Err(err) =>
                puts_error!("at" => "input", "reason" => err)
        }
    } else {
        puts_event!("input", "type" => input.type_name(), "name" => input.text());
    }
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

pub fn on_load_config(app: &mut App, config_source: &config::ConfigSource) {
    config::load_config(&app.tx, config_source);
}

pub fn on_map(app: &mut App, target: &MappingTarget, operation: &[String]) {
    use app::MappingTarget::*;

    // puts_event!("map", "target" => format!("{:?}", target), "operation" => format!("{:?}", operation));
    match *target {
        Key(ref key_sequence) =>
            app.mapping.register_key(key_sequence.clone(), operation.to_vec()),
        Mouse(ref button, ref area) =>
            app.mapping.register_mouse(*button, area.clone(), operation)
    }
}

pub fn on_multi(app: &mut App, operations: &[Operation]) {
    for op in operations {
        app.operate(op)
    }
}

pub fn on_next(app: &mut App, updated: &mut Updated, len: usize, count: Option<usize>, ignore_views: bool, move_by: MoveBy) {
    match move_by {
        MoveBy::Page =>
            updated.pointer = app.pointer.with_count(count).next(len, !ignore_views),
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

pub fn on_previous(app: &mut App, updated: &mut Updated, to_end: &mut bool, count: Option<usize>, ignore_views: bool, move_by: MoveBy) {
    match move_by {
        MoveBy::Page => {
            updated.pointer = app.pointer.with_count(count).previous(!ignore_views);
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

pub fn on_push(app: &mut App, path: String, meta: &MetaSlice) {
    if path.starts_with("http://") || path.starts_with("https://") {
        app.tx.send(Operation::PushURL(path, new_meta(meta))).unwrap();
        return;
    }

    if let Ok(path) = Path::new(&path).canonicalize() {
        if let Some(ext) = path.extension() {
            match &*ext.to_str().unwrap().to_lowercase() {
                "zip" | "rar" | "tar.gz" | "lzh" | "lha" =>
                    return archive::fetch_entries(&path, &app.encodings, app.tx.clone()),
                "pdf" =>
                    return app.tx.send(Operation::PushPdf(path.clone(), new_meta(meta))).unwrap(),
                _ => ()
            }
        }
    }

    app.operate(&Operation::PushFile(Path::new(&path).to_path_buf(), new_meta(meta)));
}

pub fn on_push_archive_entry(app: &mut App, updated: &mut Updated, archive_path: &PathBuf, entry: &ArchiveEntry) {
    updated.pointer = app.entries.push_archive_entry(&mut app.pointer, archive_path, entry);
    updated.label = true;
    app.do_show(updated);
}

pub fn on_push_http_cache(app: &mut App, updated: &mut Updated, file: &PathBuf, url: &str, meta: &MetaSlice) {
    updated.pointer = app.entries.push_http_cache(&mut app.pointer, file, url, meta);
    updated.label = true;
    app.do_show(updated);
}

pub fn on_push_path(app: &mut App, updated: &mut Updated, file: &PathBuf, meta: &MetaSlice) {
    updated.pointer = app.entries.push_path(&mut app.pointer, file, meta);
    updated.label = true;
    app.do_show(updated);
}

pub fn on_push_pdf(app: &mut App, updated: &mut Updated, file: &PathBuf, meta: &MetaSlice) {
    updated.pointer = app.entries.push_pdf(&mut app.pointer, file, meta);
    updated.label = true;
    app.do_show(updated);
}

pub fn on_push_url(app: &mut App, url: String, meta: &MetaSlice) {
    app.http_cache.fetch(url, meta);
}

pub fn on_random(app: &mut App, updated: &mut Updated, len: usize) {
    if len > 0 {
        app.pointer.current = Some(RandRange::new(0, len).ind_sample(&mut app.rng));
        updated.image = true;
    }
}

pub fn on_save(app: &mut App, path: &PathBuf, index: &Option<usize>) {
    let count = index.unwrap_or_else(|| app.pointer.counted()) - 1;
    if let Err(error) = app.gui.save(path, count) {
        puts_error!("at" => "save", "reason" => error)
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
                app.operate(&op);
            },
            Err(err) => puts_error!("at" => "scroll", "reason" => err),
        }
    }
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
    updated.label = true;
}

pub fn on_unclip(app: &mut App, updated: &mut Updated) {
    app.states.drawing.clipping = None;
    updated.image_options = true;
}

pub fn on_update_option(app: &mut App, updated: &mut Updated, option_name: &OptionName, updater: OptionUpdater) {
    use option::OptionValue;
    use operation::OptionName::*;
    use operation::OptionUpdater::*;

    {
        let value: &mut OptionValue = match *option_name {
            AutoPaging => &mut app.states.auto_paging,
            CenterAlignment => &mut app.states.view.center_alignment,
            FitTo => &mut app.states.drawing.fit_to,
            Reverse => &mut app.states.reverse,
            Scaling => &mut app.states.drawing.scaling,
            StatusBar => &mut app.states.status_bar,
            StatusFormat => &mut app.states.status_format,
            PreFetchEnabled => &mut app.states.pre_fetch.enabled,
            PreFetchLimit => &mut app.states.pre_fetch.limit_of_items,
            PreFetchPageSize => &mut app.states.pre_fetch.page_size,
            HorizontalViews => &mut app.states.view.cols,
            VerticalViews => &mut app.states.view.rows,
            ColorWindowBackground => &mut app.gui.colors.window_background,
            ColorStatusBar => &mut app.gui.colors.status_bar,
            ColorStatusBarBackground => &mut app.gui.colors.status_bar_background,
            ColorError => &mut app.gui.colors.error,
            ColorErrorBackground => &mut app.gui.colors.error_background,
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


fn on_update_views(app: &mut App, updated: &mut Updated) {
    updated.image_options = true;
    app.reset_view();
    app.pointer.set_multiplier(app.gui.len());
}
