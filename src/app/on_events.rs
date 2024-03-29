
use std::cmp::Ordering;
use std::env;
use std::fs::File;
use std::io::{Write, Read};
use std::mem::swap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::result::Result;
use std::string::ToString;
use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;

use gtk::prelude::*;
use log::trace;
use maplit::{convert_args, hashmap};
use rand::distributions::{Distribution, Uniform};

use crate::archive;
use crate::chainer;
use crate::cherenkov::Operator;
use crate::cherenkov::fill::Shape;
use crate::clipboard;
use crate::color::Color;
use crate::command_line;
use crate::config::DEFAULT_CONFIG;
use crate::controller;
use crate::editor;
use crate::entry::filter::expression::Expr as FilterExpr;
use crate::entry::{self, Meta, SearchKey, Entry, EntryContent, EntryType};
use crate::errors::{AppResultU, AppError};
use crate::events::EventName;
use crate::expandable::{Expandable, expand_all};
use crate::file_extension::get_entry_type_from_filename;
use crate::filer;
use crate::gui::Direction;
use crate::key::Key;
use crate::logger;
use crate::operation::option::{OptionName, OptionUpdater};
use crate::operation::{CherenkovParameter, ClipboardSelection, MappingTarget, MoveBy, Operation, OperationContext, ReadAs, self, SortKey, UIActionType};
use crate::option::user_switch::DummySwtich;
use crate::poppler::{PopplerDocument, self};
use crate::script;
use crate::session::{Session, write_sessions};
use crate::shell_filter;
use crate::shellexpand_wrapper as sh;
use crate::state;
use crate::util::num::range_contains;
use crate::util::path::{path_to_str, path_to_string};
use crate::util::string::prefixed_lines;

use crate::app::*;



pub fn on_app_event(app: &mut App, updated: &mut Updated, event_name: &EventName, context: &HashMap<String, String>) -> AppResultU {
    use self::EventName::*;

    let r#async = matches!(*event_name, Spawn);

    trace!("on_app_event: event={}, async={}", event_name, r#async);

    match *event_name {
        ResizeWindow => on_window_resized(app, updated)?,
        Initialize => on_initialized(app)?,
        Spawn => on_spawn(app)?,
        _ => ()
    }

    let op = Operation::Fire(Mapped::Event(event_name.clone()));
    if r#async {
        app.secondary_tx.send(op).unwrap();
    } else {
        for (k, v) in context {
            env::set_var(constant::env_name(k), v);
        }
        app.operate(op, None);
    }

    if *event_name == Quit {
        on_quit()?;
    }

    Ok(())
}

pub fn on_apng<T: AsRef<Path>>(app: &mut App, path: &T, length: u8) -> AppResultU {
    if_let_some!((entry, _) = app.current(), Ok(()));
    let imaging = app.get_imaging();
    app.cache.generate_animation_png(&entry, &imaging, length, path)
}

pub fn on_chain(target: chainer::Target) -> AppResultU {
    chainer::register(target);
    Ok(())
}

pub fn on_change_directory(path: &Expandable) -> AppResultU {
    let path = path.to_string();
    env::set_current_dir(&path)?;
    puts_event!("change_directory", "path" => o!(path));
    Ok(())
}

pub fn on_cherenkov(app: &mut App, updated: &mut Updated, parameter: &operation::CherenkovParameter, context: Option<OperationContext>) -> AppResultU {
    use crate::cherenkov::{Che, Modifier};
    use crate::cherenkov::nova::Nova;

    let context_coord = context.map(|it| it.mapped).and_then(|it| if let Mapped::Input(coord, _) = it { Some(coord) } else { None });

    let imaging = app.get_imaging();

    for (index, cell) in app.gui.cells(app.states.reverse).enumerate() {
        if let Some((entry, _)) = app.current_with(index as isize) {
            let coord = context_coord.and_then(|it| cell.get_position_on_image(&it, &app.states.drawing));
            let x = if let Some(it) = parameter.x.or_else(|| coord.as_ref().map(|it| it.x)) { it } else { continue };
            let y = if let Some(it) = parameter.y.or_else(|| coord.as_ref().map(|it| it.y)) { it } else { continue };
            app.cache.cherenkov1(
                &entry,
                &imaging,
                Modifier {
                    search_highlight: false,
                    che: Che::Nova(Nova {
                        center: (x, y),
                        color: parameter.color,
                        n_spokes: parameter.n_spokes,
                        radius: parameter.radius,
                        random_hue: parameter.random_hue,
                        seed: parameter.seed.clone(),
                        threads: parameter.threads,
                    })
                });
            updated.image = true;
        }
    }

    updated.message = true;
    app.update_message(None, false);

    Ok(())
}

pub fn on_cherenkov_reset(app: &mut App, updated: &mut Updated) -> AppResultU {
    for (index, _) in app.gui.cells(app.states.reverse).enumerate() {
        if let Some((ref entry, _)) = app.current_with(index as isize) {
            app.cache.cherenkov_reset(entry);
            updated.image_options = true;
        }
    }

    updated.message = true;
    app.update_message(None, false);

    Ok(())
}

pub fn on_clear(app: &mut App, updated: &mut Updated) -> AppResultU {
    app.entries.clear();
    app.paginator.reset();
    app.cache.clear();
    updated.image = true;
    Ok(())
}

pub fn on_clear_cache_entry(app: &mut App, updated: &mut Updated, key: &entry::Key) -> AppResultU {
    app.cache.clear_each_entry(key);
    for (index, _) in app.gui.cells(app.states.reverse).enumerate() {
        if let Some((ref entry, _)) = app.current_with(index as isize) {
            if entry.key == *key {
                updated.image = true;
                break;
            }
        }
    }
    Ok(())
}

pub fn on_clip(app: &mut App, updated: &mut Updated, inner: Region, context: Option<OperationContext>) -> AppResultU {
    let inner = extract_region_from_context(context).map(|it| it.0).unwrap_or(inner);
    let current = app.states.drawing.clipping.unwrap_or_default();
    app.states.drawing.clipping = Some(current + inner);
    updated.image_options = true;
    Ok(())
}

pub fn on_controller(app: &mut App, source: controller::Source) -> AppResultU {
    controller::register(app.secondary_tx.clone(), source)?;
    Ok(())
}

pub fn on_copy_to_clipbaord(app: &mut App, selection: ClipboardSelection) -> AppResultU {
    let cell = app.gui.cells(false).next().ok_or(AppError::Fixed("No image"))?;
    let pixbuf = cell.image.get_pixbuf().ok_or(AppError::Fixed("No static image"))?;
    clipboard::store(selection, &pixbuf);
    Ok(())
}

pub fn on_count(app: &mut App, updated: &mut Updated, count: Option<usize>) -> AppResultU {
    app.counter.set(count);
    updated.label = true;
    Ok(())
}

pub fn on_count_digit(app: &mut App, updated: &mut Updated, digit: u8) -> AppResultU {
    app.counter.push_digit(digit);
    updated.label = true;
    Ok(())
}

pub fn on_define_switch(app: &mut App, name: String, values: Vec<Vec<String>>, context: Option<OperationContext>) -> AppResultU {
    let op = app.user_switches.register(name, values)?;
    app.operate(op, context);
    Ok(())
}

pub fn on_delete(app: &mut App, updated: &mut Updated, expr: FilterExpr) -> AppResultU {
    let current_index = app.paginator.current_index();
    let app_info = app.app_info();

    let after_index = app.entries.delete(&app_info, current_index, Box::new(move |entry, app_info| expr.evaluate(entry, app_info)));

    if let Some(after_index) = after_index {
        app.paginator.update_index(Index(after_index));
    } else {
        app.paginator.reset_level();
    }

    app.update_paginator_condition();

    updated.pointer = true;
    updated.image = true;
    updated.message = true;
    Ok(())
}

pub fn on_detect_eyes(app: &mut App, parameter: CherenkovParameter) -> AppResultU {
    let mut image = vec![];
    app.gui.save(&mut image, 0)?;
    crate::cherenkov::eye_detector::detect_eyes(app.secondary_tx.clone(), parameter, image);
    Ok(())
}


pub fn on_editor(app: &mut App, editor_command: Vec<Expandable>, files: &[Expandable], sessions: &[Session], comment_out: bool, freeze: bool) -> AppResultU {
    let tx = app.secondary_tx.clone();
    let source = with_ouput_string!(out, {
        for file in files {
            let mut file = File::open(file.expand())?;
            file.read_to_string(out)?;
        }
        write_sessions(app, sessions, freeze, out);
        if comment_out {
            let mut co = prefixed_lines("# ", out);
            swap(&mut co, out);
        }
    });
    spawn(move || editor::start_edit(&tx, &editor_command, &source));
    Ok(())
}

pub fn on_error(app: &mut App, updated: &mut Updated, error: String) -> AppResultU {
    if app.error_loop_detector.in_loop(&error) {
        return Ok(());
    }

    env::set_var(constant::env_name("ERROR"), &error);
    app.update_message(Some(error), false);
    updated.message = true;
    app.fire_event(&EventName::Error);
    Ok(())
}

pub fn on_eval(app: &mut App, op: &[String], context: Option<OperationContext>) -> AppResultU {
    let op: Vec<String> = op.iter().map(|it| sh::expand_env(it)).collect();
    let op = Operation::parse_from_vec(&op)?;
    app.operate(op, context);
    Ok(())
}

pub fn on_expand(app: &mut App, updated: &mut Updated, recursive: bool, base: Option<PathBuf>) -> AppResultU {
    let count = app.counter.take();
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
        app.restore_or_first(updated, serial, false);
    }

    updated.label = true;
    Ok(())
}

pub fn on_file_changed(app: &mut App, updated: &mut Updated, path: &Path) -> AppResultU {
    env::set_var(constant::env_name("CHANGED_FILE"), path_to_string(&path));
    app.fire_event(&EventName::FileChanged);

    if !app.states.auto_reload {
        return Ok(());
    }

    let len = app.gui.len();
    let imaging = app.get_imaging();
    for delta in 0..len {
        if let Some((entry, _)) = app.current_with(delta as isize) {
            if let EntryContent::Image(ref entry_path) = entry.content {
                if entry_path == path {
                    app.cache.clear_entry(&imaging, &entry.key);
                    updated.image = true;
                }
            }
        }
    }
    Ok(())
}


#[allow(clippy::too_many_arguments)]
pub fn on_fill(app: &mut App, updated: &mut Updated, shape: Shape, region: Option<Region>, color: Color, operator: Option<Operator>, mask: bool, cell_index: usize, context: Option<OperationContext>) -> AppResultU {
    use crate::cherenkov::{Modifier, Che};

    let (mut region, cell_index) = extract_region_from_context(context)
        .or_else(|| region.map(|it| (it, cell_index)))
        .unwrap_or_else(|| (Region::full(), cell_index));

    if let Some(clipping) = app.states.drawing.clipping {
        region = region.unclipped(&clipping);
    }

    if let Some((entry, _)) = app.current_with(cell_index as isize) {
        let imaging = app.get_imaging();
        app.cache.cherenkov1(
            &entry,
            &imaging,
            Modifier {
                search_highlight: false,
                che: Che::Fill(shape, region, color, operator, mask),
            });
        updated.image = true;
    }
    Ok(())
}

pub fn on_filter(app: &mut App, updated: &mut Updated, dynamic: bool, expr: Option<FilterExpr>) -> AppResultU {
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

    app.update_message(Some(o!("Done")), false);
    Ok(())
}

pub fn on_fire(app: &mut App, mapped: &Mapped, context: Option<OperationContext>) -> AppResultU {
    if let Mapped::Input(_, ref key) = *mapped {
        if let Some(query_operation) = app.query_operation.take() {
            env::set_var(constant::env_name("query"), s!(key));
            let op = Operation::parse_from_vec(&query_operation)?;
            app.operate(op, context);
            return Ok(())
        }
    }

    let (width, height) = app.gui.window.get_size();

    if_let_some!((operations, inputs) = app.mapping.matched(mapped, width, height, true), {
        match *mapped {
            Mapped::Event(_) => (),
            Mapped::Operation(ref name, _) => return Err(AppError::UndefinedOperation(o!(name))),
            _ => puts_event!("mapped", "type" => mapped.type_name(), "name" => s!(mapped)),
        }
        Ok(())
    });

    for op in operations {
        let op = Operation::parse_from_vec(&op)?;
        let context = context.clone().unwrap_or_else(|| OperationContext { mapped: mapped.clone(), cell_index: None });
        app.operate(op, Some(context));
    }

    if let Mapped::Input(coord, _) = *mapped {
        let env = convert_args!(hashmap!("mapped" => inputs, "x" => s!(coord.x), "y" => s!(coord.y)));
        app.fire_event_with_env(&EventName::MappedInput, env);
    }
    Ok(())
}

pub fn on_first(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy) -> AppResultU {
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(false, ignore_views, count);
            updated.pointer = app.paginator.first(&paging);
        },
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).take();
            if let Some(first) = app.entries.find_nth_archive(count, false) {
                let paging = app.paging_with_index(false, ignore_views, first);
                updated.pointer = app.paginator.show(&paging);
            }
        }
    }
    Ok(())
}

pub fn on_flush_buffer(app: &mut App, updated: &mut Updated) -> AppResultU {
    let buffered = app.sorting_buffer.flush();
    push_buffered(app, updated, buffered)
}

pub fn on_fly_leaves(app: &mut App, updated: &mut Updated, n: usize) -> AppResultU {
    updated.pointer = app.paginator.set_fly_leaves(n);
    Ok(())
}

pub fn on_gif<T: AsRef<Path>>(app: &mut App, path: &T, length: u8, show: bool) -> AppResultU {
    if_let_some!((entry, _) = app.current(), Ok(()));
    let imaging = app.get_imaging();
    let tx = app.secondary_tx.clone();
    let destination = path.as_ref().to_str().map(ToString::to_string).ok_or(AppError::Fixed("WTF"))?;

    app.cache.generate_animation_gif(&entry, &imaging, length, path, move || {
        if show {
            let key = (EntryType::Image, destination.clone(), 0);
            tx.send(Operation::ClearCacheEntry(key)).unwrap();
            tx.send(Operation::PushImage(Expandable::new(destination), None, false, true, None)).unwrap();
        }
    })
}

pub fn on_go(app: &mut App, updated: &mut Updated, key: &SearchKey) -> AppResultU {
    let index = app.entries.search(key);
    if let Some(index) = index {
        if app.paginator.update_index(Index(index)) {
            updated.pointer = true;
            return Ok(());
        }
    }

    app.states.go = Some(key.clone());
    Ok(())
}

pub fn on_histoy_go(app: &mut App, updated: &mut Updated, forward: bool) -> AppResultU {
    if_let_some!((entry, _) = app.current(), Ok(()));

    loop {
        if_let_some!(key = app.history.go(forward), Err(AppError::Fixed("No history")));
        if *key == entry.key {
            continue;
        }
        if let Some(index) = app.entries.search(&SearchKey::from_key(key)) {
            updated.pointer = app.paginator.update_index(Index(index));
            return Ok(());
        }
    }
}

pub fn on_initial_process(app: &mut App, entries: Vec<command_line::Entry>, shuffle: bool, stdin_as_binary: bool) -> AppResultU {
    fn process(app: &mut App, entry: command_line::Entry, first_path: &mut Option<String>, updated: &mut Updated) -> AppResultU {
        match entry {
            CLE::Path(file) => {
                if first_path.is_none() {
                    *first_path = Some(file.clone());
                }
                on_events::on_push(app, updated, file, None, false, false)?;
            }
            CLE::Controller(source) => {
                controller::register(app.secondary_tx.clone(), source)?;
            },
            CLE::Expand(file, recursive) => {
                on_events::on_push(app, updated, file.clone(), None, false, false)?;
                app.secondary_tx.send(Operation::Expand(recursive, Some(Path::new(&file).to_path_buf()))).unwrap();
            },
            CLE::Operation(op) => {
                let op = Operation::parse_from_vec(&op)?;
                app.secondary_tx.send(op).unwrap()
            }
        }
        Ok(())
    }

    use crate::command_line::{Entry as CLE};

    app.reset_view();

    app.update_ui_visibility();

    let mut first_path = None;

    {
        let mut updated = Updated::default();
        for entry in entries {
            if let Err(err) = process(app, entry, &mut first_path, &mut updated) {
                puts_error!(err); // DONT stop entire `on_initial_process`
            }
        }
    }

    if stdin_as_binary {
        controller::stdin::register_as_binary(app.secondary_tx.clone());
    } else {
        controller::stdin::register(app.secondary_tx.clone(), app.states.history_file.clone());
    }

    if shuffle {
        let fix = first_path.map(|it| Path::new(&it).is_file()).unwrap_or(false);
        app.secondary_tx.send(Operation::Shuffle(fix)).unwrap();
    }

    app.initialize_envs_for_options();
    app.update_paginator_condition();

    app.secondary_tx.send(EventName::Initialize.operation()).unwrap();
    Ok(())
}


pub fn on_initialized(app: &mut App) -> AppResultU {
    app.secondary_tx.send(Operation::UpdateUI).unwrap();

    app.gui.register_ui_events(app.states.skip_resize_window, app.states.time_to_hide_pointer, &app.primary_tx);
    app.update_style();
    app.update_label(true, true);
    app.gui.show();
    app.update_status_bar_height(); // XXX Must Do after `gui.show`
    app.gui.refresh_status_bar_width();
    Ok(())
}

pub fn on_input(app: &mut App, mapped: &[Mapped], context: &Option<OperationContext>) -> AppResultU {
    for mapped in mapped {
        match on_fire(app, mapped, context.clone()) {
            Ok(_) => (),
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

pub fn on_jump(app: &mut App, updated: &mut Updated, name: &str, load: bool) -> AppResultU {
    use self::EntryType::*;

    let key = app.marker.get(name).ok_or(AppError::Fixed("Mark not found"))?;

    if let Some(index) = app.entries.search(&SearchKey::from_key(key)) {
        if app.paginator.update_index(Index(index)) {
            updated.pointer = true;
        }
        return Ok(());
    } else if !load {
      return Err(AppError::Fixed("Entry not found"))
    }

    let (ref entry_type, ref path, _) = *key;

    let op = match *entry_type {
        Image => Operation::PushImage(Expandable::new(path.clone()), None, false, true, None),
        Archive => Operation::PushArchive(Expandable::new(path.clone()), None, false, true),
        PDF => Operation::PushPdf(Expandable::new(path.clone()), None, false, true),
        _ => return Err(AppError::Fixed("Entry not found")),
    };

    app.secondary_tx.send(op).unwrap();
    Ok(())
}

pub fn on_kill_timer(app: &mut App, name: &str) -> AppResultU {
    app.timers.unregister(name)?;
    Ok(())
}

pub fn on_last(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy) -> AppResultU {
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(false, ignore_views, count);
            updated.pointer = app.paginator.last(&paging);
        }
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).take();
            if let Some(nth) = app.entries.find_nth_archive(count, true) {
                let paging = app.paging_with_index(false, ignore_views, nth);
                updated.pointer = app.paginator.show(&paging);
            }
        }
    }
    Ok(())
}

pub fn on_lazy_draw(app: &mut App, updated: &mut Updated, to_end: &mut bool, serial: u64, new_to_end: bool) -> AppResultU {
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
    Ok(())
}

pub fn on_link_action(app: &mut App, updated: &mut Updated, operation: &[String], context: Option<OperationContext>) -> AppResultU {
    use crate::entry::EntryContent::*;

    let mut clicked = None;
    if let Some(&Mapped::Input(ref coord, _)) = context.as_ref().map(|it| &it.mapped) {
        for (index, cell) in app.gui.cells(app.states.reverse).enumerate() {
            if let Some((entry, _)) = app.current_with(index as isize) {
                if let Some(coord) = cell.get_position_on_image(coord, &app.states.drawing) {
                    if let Pdf(ref path, index) = entry.content  {
                        let page = PopplerDocument::new_from_file(&**path).nth_page(index);
                        let links = page.get_links();
                        for link in links {
                            if coord.on_region(&link.region) {
                                clicked = Some(link.page);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(clicked) = clicked {
        let paging = app.paging_with_count(false, false, Some(clicked));
        updated.pointer = app.paginator.show(&paging);
    } else {
        let op = Operation::parse_from_vec(operation)?;
        app.operate(op, context);
    }

    Ok(())
}

pub fn on_load(app: &mut App, file: &Expandable, search_path: bool) -> AppResultU {
    let path = if search_path { file.search_path(&app.states.path_list) } else { file.expand() };
    script::load_from_file(&app.secondary_tx, &path, &app.states.path_list);
    Ok(())
}

pub fn on_load_default(app: &mut App) -> AppResultU {
    script::load(&app.secondary_tx, DEFAULT_CONFIG, &app.states.path_list);
    Ok(())
}

pub fn on_load_ui(app: &mut App, file: &Expandable, search_path: bool) -> AppResultU {
    let path = if search_path { file.search_path(&app.states.path_list) } else { file.expand() };
    app.gui.set_user_ui(&path);
    Ok(())
}

pub fn on_make_visibles(app: &mut App, regions: &[Option<Region>]) -> AppResultU {
    app.gui.make_visibles(regions);
    Ok(())
}

pub fn on_map(app: &mut App, target: MappingTarget, remain: Option<usize>, operation: Vec<String>) -> AppResultU {
    use crate::app::MappingTarget::*;

    // puts_event!("map", "target" => format!("{:?}", target), "operation" => format!("{:?}", operation));
    match target {
        Input(ref key_sequence, region) =>
            app.mapping.register_input(key_sequence, region, operation),
        Event(Some(event_name), group) =>
            app.mapping.register_event(event_name, group, remain, operation),
        Event(None, _) =>
            panic!("WTF"),
        Operation(name) =>
            app.mapping.register_operation(name, operation),
        Region(button) =>
            app.mapping.register_region(button, operation),
    }
    Ok(())
}

pub fn on_mark(app: &mut App, updated: &mut Updated, name: String, key: Option<(String, usize, Option<EntryType>)>) -> AppResultU {
    app.update_message(Some(format!("Marked with {}", name)), false);
    if let Some((path, index, entry_type)) = key {
        let entry_type = entry_type.or_else(|| {
            app.entries.search(&SearchKey { path: path.clone(), index: Some(index) }).and_then(|index| {
                app.entries.nth(index).map(|it| it.key.0)
            })
        }).ok_or(AppError::Fixed("Entry not found"))?;
        app.marker.insert(name, (entry_type, path, index));
    } else if let Some((ref entry, _)) = app.current() {
        app.marker.insert(name, entry.key.clone());
    } else {
        return Err(AppError::Fixed("Entry is empty"));
    }

    updated.label = true;
    Ok(())
}

#[allow(unused_variables)]
pub fn on_meow(app: &mut App, updated: &mut Updated) -> AppResultU {
    Ok(())
}

pub fn on_message(app: &mut App, updated: &mut Updated, message: Option<String>, keep: bool) -> AppResultU {
    updated.message = app.update_message(message, keep);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn on_move_again(app: &mut App, updated: &mut Updated, to_end: &mut bool, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool, reverse: bool) -> AppResultU {
    if (app.states.last_direction == state::Direction::Forward) ^ reverse {
        on_next(app, updated, count, ignore_views, move_by, wrap, false)
    } else {
        on_previous(app, updated, to_end, count, ignore_views, move_by, wrap, false)
    }
}

pub fn on_multi(app: &mut App, mut operations: VecDeque<Operation>, r#async: bool, context: Option<OperationContext>) -> AppResultU {
    if r#async {
        if let Some(op) = operations.pop_front() {
            app.operate(op, context);
        }
        if !operations.is_empty() {
            app.secondary_tx.send(Operation::Multi(operations, r#async))?;
        }
    } else {
        for op in operations {
            app.operate(op, context.clone());
        }
    }
    Ok(())
}

pub fn on_next(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool, remember: bool) -> AppResultU {
    if remember {
        app.states.last_direction = state::Direction::Forward;
    }
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(wrap, ignore_views, count);
            updated.pointer = app.paginator.next(&paging);
        }
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).take();
            let current = app.current();
            if let Some(next) = app.entries.find_next_archive(current, count) {
                let paging = app.paging_with_index(false, ignore_views, next);
                updated.pointer = app.paginator.show(&paging);
            }
        }
    }
    Ok(())
}

pub fn on_operate_file(app: &mut App, file_operation: &filer::FileOperation) -> AppResultU {
    use crate::entry::EntryContent::*;
    use crate::archive::ArchiveEntry;

    if let Some((entry, _)) = app.current() {
        match entry.content {
            Image(ref path) => file_operation.execute(path)?,
            Archive(_ , ArchiveEntry { ref content, .. }) => {
                let name = entry.page_filename();
                file_operation.execute_with_buffer(content, &name)?
            },
            Memory(ref content, _) => {
                let name = entry.page_filename();
                file_operation.execute_with_buffer(content, &name)?
            },
            Pdf(ref path, index) => {
                let name = entry.page_filename();
                let png = PopplerDocument::new_from_file(&**path).nth_page(index).get_png_data(&file_operation.size);
                file_operation.execute_with_buffer(png.as_ref(), &name)?
            },
            Message(ref message) =>
                return Err(AppError::Standard(o!(message)))
        };
        let text = format!("{:?}", file_operation);
        puts_event!("operate_file", "status" => "ok", "operation" => text);
    }
    Ok(())
}

pub fn on_page(app: &mut App, updated: &mut Updated, page: usize) -> AppResultU {
    if_let_some!((_, index) = app.current(), Ok(()));
    if_let_some!(found = app.entries.find_page_in_archive(index, page), Ok(()));
    updated.pointer = app.paginator.update_index(Index(found));
    Ok(())
}

pub fn on_pdf_index(app: &mut App, r#async: bool, read_operations: bool, search_path: bool, command_line: &[Expandable], fmt: poppler::index::Format, separator: Option<&str>) -> AppResultU {
    if_let_some!((entry, _) = app.current(), Ok(()));
    if let EntryContent::Pdf(ref path, _) = entry.content {
        let mut stdin = o!("");
        PopplerDocument::new_from_file(&**path).index().write(fmt, separator, &mut stdin);
        let read_as = if read_operations { ReadAs::Operations } else { ReadAs::Ignore };
        app.process_manager.call(r#async, &expand_all(command_line, search_path, &app.states.path_list), Some(stdin), read_as);
        Ok(())
    } else {
        Err(AppError::Fixed("current entry is not PDF"))
    }
}

pub fn on_pre_fetch(app: &mut App, serial: u64) -> AppResultU {
    let pre_fetch = app.states.pre_fetch.clone();
    if pre_fetch.enabled {
        trace!("on_pre_fetch: pre_fetch_serial={} serial={}", app.pre_fetch_serial, serial);

        if app.pre_fetch_serial == serial {
            let imaging = app.get_imaging();
            let size = pre_fetch.page_size as isize;
            app.pre_fetch(imaging, -size ..= size);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn on_previous(app: &mut App, updated: &mut Updated, to_end: &mut bool, count: Option<usize>, ignore_views: bool, move_by: MoveBy, wrap: bool, remember: bool) -> AppResultU {
    if remember {
        app.states.last_direction = state::Direction::Backward;
    }
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(wrap, ignore_views, count);
            updated.pointer = app.paginator.previous(&paging);
            *to_end = count.is_none() && !ignore_views;
        }
        MoveBy::Archive => {
            let count = app.counter.overwrite(count).take();
            let current = app.current();
            if let Some(previous) = app.entries.find_previous_archive(current, count) {
                let paging = app.paging_with_index(false, ignore_views, previous);
                updated.pointer = app.paginator.show(&paging);
            }
        }
    }
    Ok(())
}

pub fn on_pointer(app: &mut App, visibility: bool) -> AppResultU {
    app.gui.set_pointer_visibility(visibility);
    Ok(())
}

pub fn on_pop_count(app: &mut App) -> AppResultU {
    app.counter.pop()?;
    Ok(())
}

pub fn on_pull(app: &mut App, updated: &mut Updated) -> AppResultU {
    let buffered = app.sorting_buffer.pull_all();
    push_buffered(app, updated, buffered)
}

pub fn on_push_count(app: &mut App) -> AppResultU {
    app.counter.push();
    Ok(())
}

pub fn on_push(app: &mut App, updated: &mut Updated, path: String, meta: Option<Meta>, force: bool, show: bool) -> AppResultU {
    if is_url(&path) {
        app.secondary_tx.send(Operation::PushURL(path, meta, force, show, None))?;
        return Ok(())
    }

    on_push_path(app, updated, &Path::new(&path).to_path_buf(), meta, force, show)
}

pub fn on_push_archive<T: AsRef<Path>>(app: &mut App, path: &T, meta: Option<Meta>, force: bool, show: bool, url: Option<String>) -> AppResultU {
    archive::fetch_entries(path, meta, show, &app.encodings, app.secondary_tx.clone(), app.sorting_buffer.clone(), force, url)
}

pub fn on_push_clipboard(app: &mut App, selection: ClipboardSelection, as_operation: bool, meta: Option<Meta>, force: bool, show: bool) -> AppResultU {
    let ops = clipboard::get_operations(selection, as_operation, meta, force, show)?;
    for op in ops {
        app.secondary_tx.send(op).unwrap();
    }
    Ok(())
}

pub fn on_push_directory(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool) -> AppResultU {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushDirectory(file, meta, force));
    push_buffered(app, updated, buffered)
}

#[allow(clippy::too_many_arguments)]
pub fn on_push_image(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool, show: bool, expand_level: Option<u8>, url: Option<String>) -> AppResultU {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushImage(file, meta, force, show, expand_level, url));
    push_buffered(app, updated, buffered)
}

pub fn on_push_message(app: &mut App, updated: &mut Updated, message: String, meta: Option<Meta>, show: bool) -> AppResultU {
    let buffered = app.sorting_buffer.push_with_reserve(QueuedOperation::PushMessage(message, meta, show));
    push_buffered(app, updated, buffered)
}

pub fn on_push_memory(app: &mut App, updated: &mut Updated, buf: Vec<u8>, meta: Option<Meta>, show: bool) -> AppResultU {
    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushMemory(buf, meta, show));
    push_buffered(app, updated, buffered)
}

pub fn on_push_path<T: AsRef<Path>>(app: &mut App, updated: &mut Updated, path: &T, meta: Option<Meta>, force: bool, show: bool) -> AppResultU {
    {
        let path = if app.states.canonicalize {
            path.as_ref().canonicalize()
        } else {
            Ok(path.as_ref().to_path_buf())
        };
        if let Ok(path) = path {
            if let Some(entry_type) = get_entry_type_from_filename(&path) {
                match entry_type {
                    EntryType::Archive =>
                        return on_push_archive(app, &path, meta, force, show, None),
                    EntryType::PDF =>
                        return on_push_pdf(app, updated, path.to_path_buf(), meta, force, show, None),
                    _ =>
                        ()
                }
            }
        }
    }

    if path.as_ref().is_dir() {
        on_push_directory(app, updated, path.as_ref().to_path_buf(), meta, force)
    } else {
        on_push_image(app, updated, path.as_ref().to_path_buf(), meta, force, show, None, None)
    }
}

pub fn on_push_pdf(app: &mut App, updated: &mut Updated, file: PathBuf, meta: Option<Meta>, force: bool, show: bool, url: Option<String>) -> AppResultU {
    let document = PopplerDocument::new_from_file(&file);
    let n_pages = document.n_pages();

    let buffered = app.sorting_buffer.push_with_reserve(
        QueuedOperation::PushPdfEntries(file, n_pages, meta, force, show, url));
    push_buffered(app, updated, buffered)
}

pub fn on_push_sibling(app: &mut App, updated: &mut Updated, next: bool, clear: bool, meta: Option<Meta>, force: bool, show: bool) -> AppResultU {
    fn find_sibling<T: AsRef<Path>>(base: &T, next: bool) -> Option<PathBuf> {
        base.as_ref().parent().and_then(|dir| {
            dir.read_dir().ok().and_then(|dir| {
                let mut entries: Vec<PathBuf> = dir.filter_map(Result::ok).filter(|it| it.file_type().map(|it| it.is_file()).unwrap_or(false)).map(|it| it.path()).collect();
                entries.sort_by(|a, b| natord::compare(path_to_str(a), path_to_str(b)));
                entries.iter().position(|it| it == base.as_ref()).and_then(|found| {
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

    use crate::entry::EntryContent::*;

    let found = app.current().and_then(|(entry, _)| {
        match entry.content {
            Image(ref path) =>
                find_sibling(path, next),
            Archive(ref path, _) | Pdf(ref path, _) =>
                find_sibling(&*path.as_ref(), next),
            Memory(_, _) | Message(_) =>
                None,
        }
    });

    if let Some(found) = found {
        if clear {
            on_clear(app, updated)?;
        }
        on_push_path(app, updated, &found, meta, force, show)?;
    }
    Ok(())
}

pub fn on_push_url(app: &mut App, updated: &mut Updated, url: String, meta: Option<Meta>, force: bool, show: bool, entry_type: Option<EntryType>) -> AppResultU {
    let buffered = app.remote_cache.fetch(url, meta, force, show, entry_type);
    push_buffered(app, updated, buffered)
}

pub fn on_query(app: &mut App, updated: &mut Updated, operation: Vec<String>, caption: Option<String>) -> AppResultU {
    app.query_operation = Some(operation);
    if caption.is_some() {
        app.update_message(caption, false);
        updated.label = true;
    }
    Ok(())
}

pub fn on_queue(app: &mut App, op: Vec<String>, times: usize) -> AppResultU {
    if times == 0 {
        let op = Operation::parse_from_vec(&op)?;
        app.secondary_tx.send(op).unwrap();
    } else {
        app.secondary_tx.send(Operation::Queue(op, times - 1)).unwrap();
    }
    Ok(())
}

pub fn on_quit() -> AppResultU {
    chainer::execute();
    Ok(())
}

pub fn on_random(app: &mut App, updated: &mut Updated, len: usize) -> AppResultU {
    if len > 0 {
        let index = Uniform::new(0, len).sample(&mut app.rng);
        let paging = app.paging_with_index(false, false, index);
        app.paginator.show(&paging);
        updated.image = true;
    }
    Ok(())
}

pub fn on_record(app: &mut App, minimum_move: usize, before: usize, key: entry::Key) -> AppResultU {
    if let Some((_, current)) = app.current() {
        let d = before.checked_sub(current).unwrap_or_else(|| current - before);
        if minimum_move <= d {
            app.history.record(key);
        }
    }
    Ok(())
}

pub fn on_record_pre(app: &mut App, operation: &[String], minimum_move: usize, context: Option<OperationContext>) -> AppResultU {
    if let Some((entry, index)) = app.current() {
        app.secondary_tx.send(Operation::Record(minimum_move, index, entry.key.clone())).unwrap();
    }

    let op = Operation::parse_from_vec(operation)?;
    app.operate(op, context);
    Ok(())
}

pub fn on_refresh(app: &mut App, updated: &mut Updated, image: bool) -> AppResultU {
    if image {
        let len = app.gui.len();
        let imaging = app.get_imaging();
        for index in 0..len {
            if let Some((entry, _)) = app.current_with(index as isize) {
                app.cache.clear_entry(&imaging, &entry.key);
                updated.image = true;
            }
        }
    }
    updated.pointer = true;
    Ok(())
}

pub fn on_remove_effects(app: &mut App, updated: &mut Updated) -> AppResultU {
    if let Some((entry, _)) = app.current() {
        app.cache.uncherenkov(&entry.key);
        updated.image_options = true;
    }
    Ok(())
}

pub fn on_reset_focus(app: &mut App) -> AppResultU {
    app.gui.reset_focus();
    Ok(())
}

pub fn on_reset_scrolls(app: &mut App, to_end: bool) -> AppResultU {
    app.gui.reset_scrolls(app.states.initial_position, to_end);
    Ok(())
}

pub fn on_save(app: &mut App, path: &Path, sessions: &[Session], freeze: bool) -> AppResultU {
    let mut file = File::create(path)?;
    file.write_all(with_ouput_string!(out, write_sessions(app, sessions, freeze, out)).as_str().as_bytes())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn on_scroll(app: &mut App, direction: Direction, scroll_size: f64, crush: bool, reset_at_end: bool, operation: &[String], reset_scrolls_1: Option<Direction>, context: Option<OperationContext>) -> AppResultU {
    let saved = app.counter.clone();
    let scrolled = app.gui.scroll_views(direction, scroll_size, crush, app.counter.take(), reset_scrolls_1);

    if !scrolled && !operation.is_empty() {
        let op = Operation::parse_from_vec(operation)?;
        app.counter = saved;
        if reset_at_end {
            if let Operation::Scroll(a, b, c, d, e, _) = op {
                app.operate(Operation::Scroll(a,  b, c, d, e, Some(direction)), context);
                return Ok(());
            }
        }
        app.operate(op, context);
    }

    Ok(())
}

pub fn on_search_text(app: &mut App, updated: &mut Updated, text: Option<String>, backward: bool, color: Color) -> AppResultU {
    use crate::cherenkov::{Che, Modifier};

    fn opt_range_contains(range: &Option<Range<usize>>, index: usize, if_none: bool) -> bool {
        range.as_ref().map(|it| range_contains(it, &index)).unwrap_or(if_none)
    }

    if let Some(text) = text {
        if text.trim() == "" {
            app.update_message(None, false);
            updated.message = true;
            return Ok(());
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

    if_let_some!(text = app.search_text.clone(), ok!(app.update_message(Some(o!("Empty")), false)));

    let seq: Vec<(usize, Arc<Entry>)> = if backward {
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
                    doc = Some(Rc::clone(p_doc));
                }
            }

            if doc.is_none() {
                let d = Rc::new(PopplerDocument::new_from_file(&**path));
                doc = Some(Rc::clone(&d));
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

            let imaging = app.get_imaging();

            app.cache.clear_entry_search_highlights(&entry);
            let modifiers: Vec<Modifier> = regions.iter().map(|region| Modifier { search_highlight: true, che: Che::Fill(Shape::Rectangle, *region, color, None, false) }).collect();
            app.cache.cherenkov(
                &entry,
                &imaging,
                modifiers.as_slice());

            if new_found_on.is_none() {
                updated.pointer = app.paginator.update_index(Index(index));
                updated.image = true;
                app.update_message(Some(o!("Found!")), false);
                let left = index / cells * cells;
                new_found_on = Some(left .. left + cells - 1);
            }
        }
    }

    if new_found_on.is_none() {
        app.update_message(Some(o!("Not found!")), false);
    } else {
        updated.target_regions = Some(first_regions);
    }
    app.found_on = new_found_on;

    Ok(())
}

pub fn on_set_env(_: &mut App, name: &str, value: &Option<String>) -> AppResultU {
    if let Some(ref value) = *value {
        env::set_var(name, value);
    } else {
        env::remove_var(name);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn on_shell(app: &mut App, r#async: bool, read_as: ReadAs, search_path: bool, command_line: &[Expandable], sessions: &[Session], freeze: bool) -> AppResultU {
    let stdin = if !sessions.is_empty() {
        Some(with_ouput_string!(out, write_sessions(app, sessions, freeze, out)))
    } else {
        None
    };

    app.update_counter_env(true);
    app.process_manager.call(r#async, &expand_all(command_line, search_path, &app.states.path_list), stdin, read_as);
    Ok(())
}

pub fn on_shell_filter(app: &mut App, command_line: &[Expandable], search_path: bool) -> AppResultU {
    app.update_counter_env(true);
    shell_filter::start(expand_all(command_line, search_path, &app.states.path_list), app.secondary_tx.clone());
    Ok(())
}

pub fn on_show(app: &mut App, updated: &mut Updated, count: Option<usize>, ignore_views: bool, move_by: MoveBy) -> AppResultU {
    match move_by {
        MoveBy::Page => {
            let paging = app.paging_with_count(false, false, count);
            updated.pointer = app.paginator.show(&paging);
        },
        MoveBy::Archive => {
            on_first(app, updated, count, ignore_views, move_by)?;
        }
    }
    Ok(())
}

pub fn on_show_command_line(app: &mut App, initial: &str) -> AppResultU {
    app.gui.show_command_line(initial, &app.secondary_tx)
}

pub fn on_shuffle(app: &mut App, updated: &mut Updated, fix_current: bool) -> AppResultU {
    let serial = app.store();
    let app_info = app.app_info();
    app.entries.shuffle(&app_info);

    if fix_current {
        app.restore_or_first(updated, serial, false);
        updated.image = 1 < app.gui.len();
    } else {
        updated.image = true;
        updated.pointer = true;
    }
    updated.label = true;
    Ok(())
}

pub fn on_sort(app: &mut App, updated: &mut Updated, fix_current: bool, sort_key: SortKey, reverse: bool) -> AppResultU {
    use self::SortKey::*;

    let serial = app.store();
    let app_info = app.app_info();

    if sort_key == SortKey::Natural && !reverse {
        app.entries.sort(&app_info);
    } else {
        app.entries.sort_by(&app_info, move |a, b| {
            if sort_key == Natural {
                return maybe_reverse(reverse, entry::compare_key(&a.key, &b.key));
            }

            a.info.lazy(&a.content, |ai| {
                b.info.lazy(&b.content, |bi| {
                    let result = match sort_key {
                        Natural => panic!("WTF!"),
                        FileSize =>
                            ai.file_size.cmp(&bi.file_size),
                        Created =>
                            ai.created.cmp(&bi.created),
                        Accessed =>
                            ai.accessed.cmp(&bi.accessed),
                        Modified =>
                            ai.modified.cmp(&bi.modified),
                        Dimensions =>
                            ai.dimensions.cmp(&bi.dimensions),
                        Height =>
                            ai.dimensions.map(|it| it.height).cmp(&bi.dimensions.map(|it| it.height)),
                        Width =>
                            ai.dimensions.map(|it| it.height).cmp(&bi.dimensions.map(|it| it.height)),
                    };
                    maybe_reverse(reverse, result)
                })
            })
        });
    }

    if fix_current {
        app.restore_or_first(updated, serial, false);
        updated.image = 1 < app.gui.len();
    } else {
        updated.image = true;
        updated.pointer = true;
    }

    Ok(())
}

pub fn on_sorter(app: &mut App, updated: &mut Updated, fix_current: bool, sorter_command: &[Expandable], reverse: bool) -> AppResultU {
    use std::process::{Command, Stdio};

    let output = {
        let mut input = o!("");

        for entry in app.entries.iter() {
            let key = &entry.key;
            sprintln!(input, "{}\t{}\t{}", key.0, key.1, key.2);
        }

        let (head, args) = sorter_command.split_first().ok_or("Empty command")?;
        let mut command = Command::new(&head.expand());
        for arg in args {
            command.arg(&arg.expand());
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = command.spawn()?;
        child.stdin.ok_or("No stdin")?.write_all(input.as_bytes())?;
        let mut output = o!("");
        child.stdout.ok_or("Not stdout")?.read_to_string(&mut output)?;

        output
    };

    let orders = tap!(mut orders = HashMap::new(), for (index, line) in output.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        if columns.len() != 3 {
            return Err("Invalid format".into());
        }
        let key: entry::Key = (columns[0].parse()?, o!(columns[1]), columns[2].parse()?);
        orders.insert(key, index);
    });

    let removed = {
        let mut removes = HashSet::new();
        for (index, entry) in app.entries.iter().enumerate() {
            if !orders.contains_key(&entry.key) {
                removes.insert(index);
            }
        }

        tap!(removed = !removes.is_empty(), if removed {
            let app_info = app.app_info();
            app.entries.remove(&app_info, &removes);
            app.update_paginator_condition();
        })
    };

    let serial = app.store();

    {
        let app_info = app.app_info();
        app.entries.sort_by(&app_info, move |a, b| {
            let a = orders.get(&a.key);
            let b = orders.get(&b.key);
            maybe_reverse(reverse, a.cmp(&b))
        });
    }

    if fix_current {
        app.restore_or_first(updated, serial, false);
        updated.image = 1 < app.gui.len() || removed;
    } else {
        updated.image = true;
        updated.pointer = true;
    }

    Ok(())
}

pub fn on_spawn(app: &mut App) -> AppResultU {
    app.states.spawned = true;
    app.gui.refresh_status_bar_width();
    app.operate(Operation::Draw, None);
    Ok(())
}

pub fn on_tell_region(app: &mut App, left: f64, top: f64, right: f64, bottom: f64, button: &Key) -> AppResultU {
    let (mx, my) = (left as i32, top as i32);
    for (index, cell) in app.gui.cells(app.states.reverse).enumerate() {
        if app.current_with(index as isize).is_some() {
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
                let op = Operation::Fire(Mapped::Region(region, button.clone(), index));
                app.secondary_tx.send(op).unwrap();
            }
        }
    }
    Ok(())
}

pub fn on_timer(app: &mut App, name: Option<String>, op: Vec<String>, interval: Duration, repeat: Option<usize>, r#async: bool) -> AppResultU {
    app.timers.register(name, op, interval, repeat, r#async)
}

pub fn on_unchain(target: &chainer::Target) -> AppResultU {
    chainer::unregister(target);
    Ok(())
}

pub fn on_ui_action(app: &mut App, action_type: UIActionType) -> AppResultU {
    use self::UIActionType::*;
    use crate::gui::Screen::*;

    let mut result = Ok(());

    match action_type {
        SendOperation => {
            result = app.gui.pop_operation_entry().map(|op| {
                if let Some(op) = op {
                    app.secondary_tx.send(op).unwrap();
                }
            });
            app.states.screen = Main;
        },
        Close => {
            app.states.screen = Main;
        }
    }

    app.update_ui_visibility();
    result
}

pub fn on_unclip(app: &mut App, updated: &mut Updated) -> AppResultU {
    app.states.drawing.clipping = None;
    updated.image_options = true;
    Ok(())
}

pub fn on_undo(app: &mut App, updated: &mut Updated, count: Option<usize>) -> AppResultU {
    // `counted` should be evaluated
    #[allow(clippy::or_fun_call)]
    let count = count.unwrap_or(app.counter.take());

    if let Some((ref entry, _)) = app.current() {
        app.cache.undo_cherenkov(&entry.key, count)
    }
    updated.image_options = true;
    Ok(())
}

pub fn on_unmap(app: &mut App, target: &MappingTarget) -> AppResultU {
    use crate::app::MappingTarget::*;

    // puts_event!("unmap", "target" => format!("{:?}", target), "operation" => format!("{:?}", operation));
    match *target {
        Input(ref key_sequence, ref region) =>
            app.mapping.unregister_input(key_sequence, region),
        Event(ref event_name, ref group) =>
            app.mapping.unregister_event(event_name, group),
        Operation(ref name) =>
            app.mapping.unregister_operation(name),
        Region(ref button) =>
            app.mapping.unregister_region(button),
    }
    Ok(())
}

pub fn on_unmark(app: &mut App, target: &Option<String>) -> AppResultU {
    match *target {
        Some(ref target) => {
            if app.marker.remove(target).is_none() {
               return Err(AppError::Fixed("Mark not found"))
            }
        },
        None => app.marker.clear(),
    }
    Ok(())
}

pub fn on_update_option(app: &mut App, updated: &mut Updated, option_name: &OptionName, updater: &OptionUpdater) -> AppResultU {
    use crate::option::OptionValue;
    use crate::operation::option::OptionName::*;
    use crate::operation::option::OptionUpdater::*;
    use crate::operation::option::PreDefinedOptionName::*;
    use crate::size;

    let mut dummy_switch = DummySwtich::new();
    let freezed = app.states.freezed;

    {
        let do_update_fix_to = match *updater {
            Increment(_) | Decrement(_) if *option_name == PreDefined(FitTo) => {
                !matches!(app.states.drawing.fit_to, size::FitTo::Scale(_))
            },
            _ => false,
        };

        let value: &mut dyn OptionValue = match *option_name {
            PreDefined(ref option_name) => match *option_name {
                AbbrevLength => &mut app.states.abbrev_length,
                Animation => &mut app.states.drawing.animation,
                AutoReload => &mut app.states.auto_reload,
                AutoPaging => &mut app.states.auto_paging,
                Canonicalize => &mut app.states.canonicalize,
                CurlConnectTimeout => &mut app.states.curl_options.connect_timeout,
                CurlFollowLocation => &mut app.states.curl_options.follow_location,
                CurlLowSpeedLimit => &mut app.states.curl_options.low_speed_limit,
                CurlLowSpeedTime => &mut app.states.curl_options.low_speed_time,
                CurlTimeout => &mut app.states.curl_options.timeout,
                EmptyStatusFormat => &mut app.states.empty_status_format,
                FitTo => &mut app.states.drawing.fit_to,
                Freeze => &mut app.states.freezed,
                HistoryFile => &mut app.states.history_file,
                HorizontalFlip => &mut app.states.drawing.horizontal_flip,
                HorizontalViews => &mut app.states.view.cols,
                IdleTime => &mut app.states.idle_time,
                IgnoreFailures => &mut app.states.ignore_failures,
                InitialPosition => &mut app.states.initial_position,
                LogFile => &mut app.states.log_file,
                MaskOperator => &mut app.states.drawing.mask_operator,
                PathList => &mut app.states.path_list,
                PreFetchEnabled => &mut app.states.pre_fetch.enabled,
                PreFetchLimit => &mut app.states.pre_fetch.limit_of_items,
                PreFetchPageSize => &mut app.states.pre_fetch.page_size,
                PreFetchStages => &mut app.states.pre_fetch.cache_stages,
                Reverse => &mut app.states.reverse,
                Rotation => &mut app.states.drawing.rotation,
                Screen => &mut app.states.screen,
                SkipResizeWindow => &mut app.states.skip_resize_window,
                StablePush => &mut app.states.stable_push,
                StatusBar => &mut app.states.status_bar,
                StatusBarAlign => &mut app.states.status_bar_align,
                StatusBarHeight => &mut app.states.status_bar_height,
                StatusBarOverlay => &mut app.states.status_bar_overlay,
                StatusFormat => &mut app.states.status_format,
                StdOut => &mut app.states.stdout,
                Style => &mut app.states.style,
                TimeToHidePointer => &mut app.states.time_to_hide_pointer,
                TitleFormat => &mut app.states.title_format,
                UpdateCacheAccessTime => &mut app.states.update_cache_atime,
                VerticalFlip => &mut app.states.drawing.vertical_flip,
                VerticalViews => &mut app.states.view.rows,
                WatchFiles => &mut app.states.watch_files,
                ColorLink => &mut app.states.drawing.link_color,
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


        if do_update_fix_to {
            value.set(&format!("{}%", (app.current_base_scale.unwrap_or(1.0) * 100.00) as usize)).unwrap();
        }

        match *updater {
            Cycle(ref reverse, ref candidates) => value.cycle(*reverse, app.counter.take(), candidates)?,
            Disable => value.disable()?,
            Enable => value.enable()?,
            Set(ref arg) => value.set(arg)?,
            Toggle => value.toggle()?,
            Unset => value.unset()?,
            SetByCount => value.set_from_count(app.counter.take_option())?,
            Increment(delta) => value.increment(app.counter.take_option().unwrap_or(delta))?,
            Decrement(delta) => value.decrement(app.counter.take_option().unwrap_or(delta))?,
        }
    }

    fix_option_values(app);

    updated.image = true;

    if let PreDefined(ref option_name) = *option_name {
        app.update_env_for_option(option_name);
        if option_name.is_for_curl() {
            app.remote_cache.update_curl_options(app.states.curl_options.clone());
        }
        match *option_name {
            AutoReload | WatchFiles =>
                app.update_watcher(),
            AbbrevLength =>
                updated.label = true,
            Freeze if freezed && !app.states.freezed =>
                updated.image = true,
            IgnoreFailures =>
                app.remote_cache.set_ignore_failures(app.states.ignore_failures),
            StablePush =>
                app.sorting_buffer.set_stability(app.states.stable_push),
            StatusBar | StatusBarOverlay => {
                app.update_ui_visibility();
                updated.size = true;
            },
            Screen =>
                app.update_ui_visibility(),
            StatusBarAlign =>
                app.gui.set_status_bar_align(app.states.status_bar_align.0),
            StatusBarHeight => {
                app.update_status_bar_height();
                updated.size = true;
            }
            FitTo | Rotation | HorizontalFlip | VerticalFlip =>
                updated.size = true,
            PreFetchLimit =>
                app.cache.update_limit(app.states.pre_fetch.limit_of_items),
            PreFetchStages =>
                app.cache.update_stages(app.states.pre_fetch.cache_stages),
            Style =>
                app.update_style(),
            TimeToHidePointer =>
                app.gui.set_time_to_hide_pointer(app.states.time_to_hide_pointer),
            Animation | ColorLink => {
                app.cache.clear();
                updated.image = true;
            },
            VerticalViews | HorizontalViews =>
                on_update_views(app, updated, false)?,
            UpdateCacheAccessTime =>
                app.remote_cache.do_update_atime = app.states.update_cache_atime,
            _ => ()
        }
    }
    Ok(())
}

pub fn on_user(_: &mut App, data: &[(String, String)]) -> AppResultU {
    let mut pairs = vec![(o!("event"), o!("user"))];
    pairs.extend_from_slice(data);
    logger::puts(&pairs);
    Ok(())
}

pub fn on_views(app: &mut App, updated: &mut Updated, cols: Option<usize>, rows: Option<usize>, ignore_views: bool) -> AppResultU {
    use crate::operation::option::PreDefinedOptionName::*;

    if let Some(cols) = cols {
        app.states.view.cols = cols;
        app.update_env_for_option(&HorizontalViews);
    }
    if let Some(rows) = rows {
        app.states.view.rows = rows;
        app.update_env_for_option(&VerticalViews);
    }
    on_update_views(app, updated, ignore_views)
}

pub fn on_views_fellow(app: &mut App, updated: &mut Updated, for_rows: bool, ignore_views: bool) -> AppResultU {
    use crate::operation::option::PreDefinedOptionName::*;

    let count = app.counter.take();
    if for_rows {
        app.states.view.rows = count;
        app.update_env_for_option(&VerticalViews);
    } else {
        app.states.view.cols = count;
        app.update_env_for_option(&HorizontalViews);
    };
    on_update_views(app, updated, ignore_views)
}

pub fn on_wakeup_timer(app: &mut App, name: &str) -> AppResultU {
    app.timers.wakeup(name);
    Ok(())
}

pub fn on_when(app: &mut App, filter: FilterExpr, unless: bool, op: &[String], context: Option<OperationContext>) -> AppResultU {
    let app_info = app.app_info();
    if_let_some!((_, index, _) = app.current_non_fly_leave(), Ok(()));
    if_let_some!(r = app.entries.validate_nth(index, filter, &app_info), Ok(()));

    if r ^ unless {
        let op = Operation::parse_from_vec(op)?;
        app.operate(op, context);
    }
    Ok(())
}

pub fn on_window_resized(app: &mut App, updated: &mut Updated) -> AppResultU {
    updated.size = true;
    // Ignore followed PreFetch
    app.pre_fetch_serial += 1;
    app.gui.refresh_status_bar_width();
    Ok(())
}

pub fn on_with_message(app: &mut App, updated: &mut Updated, message: Option<String>, op: Operation, context: Option<OperationContext>) -> AppResultU {
    updated.message = true;
    app.update_message(message, false);
    app.secondary_tx.send(Operation::UpdateUI)?;
    if let Some(context) = context {
        app.secondary_tx.send(Operation::Context(context, Box::new(op)))?;
    } else {
        app.secondary_tx.send(op)?;
    }
    Ok(())
}

pub fn on_write<T: AsRef<Path>>(app: &mut App, path: &T, index: &Option<usize>) -> AppResultU {
    let count = index.unwrap_or_else(|| app.counter.take()) - 1;
    app.gui.save_to_file(path, count)?;
    Ok(())
}


fn extract_region_from_context(context: Option<OperationContext>) -> Option<(Region, usize)> {
    if let Some(Mapped::Region(ref region, _, cell_index)) = context.map(|it| it.mapped) {
        return Some((*region, cell_index));
    }
    None
}

fn is_url(path: &str) -> bool {
    if_let_some!(index = path.find("://"), false);
    index < 10
}

fn on_update_views(app: &mut App, updated: &mut Updated, ignore_views: bool) -> AppResultU {
    updated.size = true;
    let serial = app.store();
    app.reset_view();
    app.restore_or_first(updated, serial, ignore_views);
    Ok(())
}

fn push_buffered(app: &mut App, updated: &mut Updated, ops: Vec<QueuedOperation>) -> AppResultU {
    fn gen_target<T: AsRef<Path>>(show: bool, url: &Option<String>, path: &T) -> Option<ShowTarget> {
        if show {
            url.clone().map(ShowTarget::Url).or_else(|| path.as_ref().canonicalize().ok().map(ShowTarget::File))
        } else {
            None
        }
    }

    enum ShowTarget {
        Index(usize),
        File(PathBuf),
        Url(String),
    }

    use crate::operation::QueuedOperation::*;

    let before_len = app.entries.len();
    let app_info = app.app_info();
    let mut last_show_target = None;

    for op in ops {
        let len = app.entries.len();
        let mut show_target = None;

        match op {
            PushImage(path, meta, force, show, expand_level, url) => {
                show_target = gen_target(show, &url, &path);
                app.entries.push_image(&app_info, &path, meta, force, expand_level, url)?;
            },
            PushDirectory(path, meta, force) =>
                app.entries.push_directory(&app_info, &path, &meta, force)?,
            PushArchive(archive_path, meta, force, show, url) =>
                on_push_archive(app, &archive_path, meta, force, show, url)?,
            PushArchiveEntry(archive_path, entry, meta, force, show, url) => {
                show_target = gen_target(show, &url, &archive_path);
                app.entries.push_archive_entry(&app_info, &archive_path, &entry, meta, force, url);
            },
            PushMemory(buf, meta, show) => {
                app.entries.push_memory(&app_info, buf, meta, false, None)?;
                if show {
                    show_target = Some(ShowTarget::Index(len))
                }
            },
            PushPdf(pdf_path, meta, force, show, url) =>
                on_push_pdf(app, updated, pdf_path, meta, force, show, url)?,
            PushPdfEntries(pdf_path, pages, meta, force, show, url) => {
                show_target = gen_target(show, &url, &pdf_path);
                let pdf_path = Arc::new(pdf_path.clone());
                for index in 0 .. pages {
                    app.entries.push_pdf_entry(&app_info, &pdf_path, index, meta.clone(), force, url.clone());
                }
            },
            PushMessage(message, meta, show) => {
                if show {
                    show_target = Some(ShowTarget::Index(len))
                }
                app.entries.push_message(&app_info, message, meta);
            },
        }

        if show_target.is_some() {
            last_show_target = show_target;
        }

        updated.label = true;
    }

    app.update_paginator_condition();
    app.remote_cache.update_sorting_buffer_len();

    if let Some(show_target) = last_show_target {
        let index = match show_target {
            ShowTarget::File(path) => {
                path.to_str().and_then(|path| {
                    let key = entry::SearchKey { path: o!(path), index: None };
                    app.entries.search(&key)
                })
            },
            ShowTarget::Url(url) => {
                let key = entry::SearchKey { path: url, index: None };
                app.entries.search(&key)
            },
            ShowTarget::Index(index) =>
                Some(index)
        };
        if let Some(index) = index {
            let paging = app.paging_with_count(false, false, Some(index + 1));
            updated.pointer = app.paginator.show(&paging);
        }
    } else if before_len == 0 && 0 < app.entries.len() {
        updated.pointer |= app.paginator.reset_level()
    }

    app.do_go(updated);
    Ok(())
}

fn maybe_reverse(reverse: bool, original: Ordering) -> Ordering {
    if reverse {
        match original {
            Ordering::Greater => Ordering::Less,
            Ordering::Less => Ordering::Greater,
            other => other,
        }
    } else {
        original
    }
}

fn fix_option_values(app: &mut App) {
    fn non_zero(v: &mut usize) {
        if *v == 0 {
            *v = 1;
        }
    }

    non_zero(&mut app.states.view.rows);
    non_zero(&mut app.states.view.cols);
}
