
use std::collections::{HashSet, HashMap, VecDeque};
use std::env;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{sleep, spawn};
use std::time::Duration;

use encoding::types::EncodingRef;
use gtk::prelude::*;
use libc;
use rand::{self, ThreadRng};

use command_line::Initial;
use config;
use constant;
use counter::Counter;
use entry::image::Imaging;
use entry::{Entry, EntryContainer, EntryContent, Serial, Key};
use error_channel;
use events::EventName;
use gui::{Gui, Screen};
use history::History;
use image_cache::ImageCache;
use image_fetcher::ImageFetcher;
use logger;
use mapping::{Mapping, Mapped};
use operation::option::PreDefinedOptionName;
use operation::{Operation, QueuedOperation, OperationContext, MappingTarget, MoveBy, Updated};
use option::user_switch::UserSwitchManager;
use paginator::values::Index;
use paginator::{self, Paginator, Paging};
use remote_cache::RemoteCache;
use script;
use size::{Size, FitTo, Region};
use sorting_buffer::SortingBuffer;
use state::{AutoPaging, States, PreFetchState};
use termination;
use timer::TimerManager;
use util::path::path_to_str;
use watcher::Watcher;

mod error_loop_detector;
mod on_events;
pub mod info;

use self::info::AppInfo;



pub struct App {
    pub cache: ImageCache,
    pub entries: EntryContainer,
    pub gui: Gui,
    pub history: History,
    pub log: logger::memory::Memory,
    pub mapping: Mapping,
    pub marker: HashMap<String, Key>,
    pub paginator: Paginator,
    pub primary_tx: Sender<Operation>,
    pub query_operation: Option<Vec<String>>,
    pub remote_cache: RemoteCache,
    pub states: States,
    pub timers: TimerManager,
    pub tx: Sender<Operation>,
    counter: Counter,
    current_base_scale: Option<f64>, // Scale of first scaled image
    current_env_keys: HashSet<String>,
    do_clear_cache: bool,
    draw_serial: u64,
    encodings: Vec<EncodingRef>,
    error_loop_detector: error_loop_detector::Detector,
    fetcher: ImageFetcher,
    found_on: Option<Range<usize>>,
    last_message: Option<String>,
    pre_fetch_serial: u64,
    rng: ThreadRng,
    search_text: Option<String>,
    sorting_buffer: SortingBuffer<QueuedOperation>,
    user_switches: UserSwitchManager,
    watcher: Watcher,
}


impl App {
    pub fn new(mut initial: Initial) -> (App, Receiver<Operation>, Receiver<Operation>) {
        let (tx, rx) = channel();
        let (primary_tx, primary_rx) = channel();

        let mut states = States::default();

        if initial.enforce_gtk_theme {
            env::set_var("GTK_THEME", "Adwaita:light");
        }

        if initial.encodings.is_empty() {
            use encoding::all::*;
            initial.encodings.push(UTF_8);
            initial.encodings.push(WINDOWS_31J);
        }

        if initial.silent {
            states.stdout.unregister();
        } else {
            states.stdout.register();
        }

        set_envs();

        let cache_limit = PreFetchState::default().limit_of_items;
        let cache = ImageCache::new(cache_limit);

        let sorting_buffer = SortingBuffer::new();

        let app = App {
            cache: cache.clone(),
            counter: Counter::default(),
            current_base_scale: None,
            current_env_keys: HashSet::new(),
            do_clear_cache: false,
            draw_serial: 0,
            encodings: initial.encodings.clone(),
            entries: EntryContainer::new(),
            error_loop_detector: error_loop_detector::Detector::default(),
            fetcher: ImageFetcher::new(cache),
            found_on: None,
            gui: Gui::new(&initial.window_role),
            history: History::default(),
            last_message: None,
            log: logger::memory::Memory::new(),
            mapping: Mapping::new(),
            marker: HashMap::new(),
            paginator: Paginator::new(),
            pre_fetch_serial: 0,
            primary_tx,
            query_operation: None,
            remote_cache: RemoteCache::new(initial.curl_threads, tx.clone(), sorting_buffer.clone()),
            rng: rand::thread_rng(),
            search_text: None,
            sorting_buffer,
            states,
            timers: TimerManager::new(tx.clone()),
            tx: tx.clone(),
            user_switches: UserSwitchManager::new(tx.clone()),
            watcher: Watcher::new(tx.clone()),
        };

        if initial.load_config {
            script::load(&app.tx, &config::get_config_source(initial.config_file.as_ref()), &app.states.path_list);
        }
        app.tx.send(Operation::InitialProcess(initial.entries, initial.shuffle, initial.stdin_as_binary)).unwrap();
        error_channel::register(app.tx.clone());

        (app, primary_rx, rx)
    }

    pub fn fire_event_with_env(&mut self, event_name: &EventName, env: HashMap<String, String>) {
        use self::EventName::*;

        let op = event_name.operation_with_env(env);

        match *event_name {
            Initialize => self.tx.send(op).unwrap(),
            _ => self.operate(op, None),
        }
    }

    pub fn fire_event(&mut self, event_name: &EventName) {
        self.fire_event_with_env(event_name, HashMap::new())
    }

    pub fn operate(&mut self, operation: Operation, context: Option<OperationContext>) {
        use self::Operation::*;
        use self::on_events::*;

        let operation_name = d!(operation);
        trace!("operate_with_context: operation={:?}", operation_name);

        let mut updated = Updated::default();
        let mut to_end = false;
        let len = self.entries.len();
        let count = self.counter.peek();

        {
            let operated = match operation {
                AppEvent(ref event_name, ref context) =>
                    on_app_event(self, &mut updated, event_name, context),
                Backward =>
                    on_histoy_go(self, &mut updated, false),
                ChangeDirectory(ref path) =>
                    on_change_directory(path),
                Cherenkov(ref parameter) =>
                    on_cherenkov(self, &mut updated, parameter, context),
                CherenkovReset =>
                    on_cherenkov_reset(self, &mut updated),
                Clear =>
                    on_clear(self, &mut updated),
                Clip(region) =>
                    on_clip(self, &mut updated, region, context),
                Context(context, op) =>
                    return self.operate(*op, Some(context)),
                Controller(source) =>
                    on_controller(self, source),
                CopyToClipboard(selection) =>
                    on_copy_to_clipbaord(self, selection),
                Count(count) =>
                    on_count(self, &mut updated, count),
                CountDigit(digit) =>
                    on_count_digit(self, &mut updated, digit),
                DefineUserSwitch(name, values) =>
                    on_define_switch(self, name, values, context),
                Delete(expr) =>
                    on_delete(self, &mut updated, *expr),
                Draw =>
                    ok!(updated.image = true),
                Editor(ref editor_command, ref files, ref sessions, comment_out) =>
                   on_editor(self, editor_command.clone(), files, sessions, comment_out),
                Error(error) =>
                    on_error(self, &mut updated, error),
                Eval(ref op) =>
                    on_eval(self, op, context),
                Expand(recursive, ref base) =>
                    on_expand(self, &mut updated, recursive, base.clone()),
                FileChanged(ref path) =>
                    on_file_changed(self, &mut updated, path),
                Fill(shape, region, color, operator, mask, cell_index) =>
                    on_fill(self, &mut updated, shape, region, color, operator, mask, cell_index, context),
                Filter(dynamic, expr) =>
                    on_filter(self, &mut updated, dynamic, *expr),
                Fire(ref mapped) =>
                    on_fire(self, mapped, context),
                First(count, ignore_views, move_by, _) =>
                    on_first(self, &mut updated, count, ignore_views, move_by),
                FlushBuffer =>
                    on_flush_buffer(self, &mut updated),
                FlyLeaves(n) =>
                    on_fly_leaves(self, &mut updated, n),
                Forward =>
                    on_histoy_go(self, &mut updated, true),
                Go(ref key) =>
                    on_go(self, &mut updated, key),
                InitialProcess(entries, shuffle, stdin_as_binary) =>
                    on_initial_process(self, entries, shuffle, stdin_as_binary),
                Input(ref mapped) =>
                    on_input(self, mapped, &context),
                Jump(ref name, load) =>
                    on_jump(self, &mut updated, name, load),
                KillTimer(ref name) =>
                    on_kill_timer(self, name),
                Last(count, ignore_views, move_by, _) =>
                    on_last(self, &mut updated, count, ignore_views, move_by),
                LazyDraw(serial, new_to_end) =>
                    on_lazy_draw(self, &mut updated, &mut to_end, serial, new_to_end),
                LinkAction(ref operation) =>
                    on_link_action(self, &mut updated, operation, context),
                Load(ref file, search_path) =>
                    on_load(self, file, search_path),
                LoadDefault =>
                    on_load_default(self),
                LoadUI(ref file, search_path) =>
                    on_load_ui(self, file, search_path),
                MakeVisibles(ref regions) =>
                    on_make_visibles(self, regions),
                Map(target, remain, mapped_operation) =>
                    on_map(self, target, remain, mapped_operation),
                Mark(name, key) =>
                    on_mark(self, &mut updated, name, key),
                Meow =>
                    on_meow(self, &mut updated),
                Message(message, keep) =>
                    on_message(self, &mut updated, message, keep),
                MoveAgain(count, ignore_views, move_by, wrap, reverse) =>
                    on_move_again(self, &mut updated, &mut to_end, count, ignore_views, move_by, wrap, reverse),
                Multi(ops, async) =>
                    on_multi(self, ops, async, context),
                Next(count, ignore_views, move_by, wrap, remember) =>
                    on_next(self, &mut updated, count, ignore_views, move_by, wrap, remember),
                Nop =>
                    Ok(()),
                OperateFile(ref file_operation) =>
                    on_operate_file(self, file_operation),
                Page(page) =>
                    on_page(self, &mut updated, page),
                PdfIndex(async, read_operations, search_path, ref command_line, fmt, ref separator) =>
                    on_pdf_index(self, async, read_operations, search_path, command_line, fmt, separator.as_ref().map(String::as_str)),
                PreFetch(pre_fetch_serial) =>
                    on_pre_fetch(self, pre_fetch_serial),
                Previous(count, ignore_views, move_by, wrap, remember) =>
                    on_previous(self, &mut updated, &mut to_end, count, ignore_views, move_by, wrap, remember),
                PopCount =>
                    on_pop_count(self),
                Pull =>
                    on_pull(self, &mut updated),
                Push(path, meta, force) =>
                    on_push(self, &mut updated, path.to_string(), meta, force),
                PushCount =>
                    on_push_count(self),
                PushArchive(file, meta, force) =>
                    on_push_archive(self, &file.expand(), meta, force, None),
                PushClipboard(selection, as_operation, meta, force) =>
                    on_push_clipboard(self, selection, as_operation, meta, force),
                PushDirectory(file, meta, force) =>
                    on_push_directory(self, &mut updated, file.expand(), meta, force),
                PushImage(file, meta, force, expand_level) =>
                    on_push_image(self, &mut updated, file.expand(), meta, force, expand_level, None),
                PushMemory(buf, meta) =>
                    on_push_memory(self, &mut updated, buf, meta),
                PushPdf(file, meta, force) =>
                    on_push_pdf(self, &mut updated, file.expand(), meta, force, None),
                PushSibling(next, meta, force, show) =>
                    on_push_sibling(self, &mut updated, next, meta, force, show),
                PushURL(url, meta, force, entry_type) =>
                    on_push_url(self, &mut updated, url, meta, force, entry_type),
                Query(operation, caption) =>
                    on_query(self, &mut updated, operation, caption),
                Random =>
                    on_random(self, &mut updated, len),
                Record(minimum_move, position, key) =>
                    on_record(self, minimum_move, position, key),
                RecordPre(op, minimum_move) =>
                    on_record_pre(self, &op, minimum_move, context),
                Refresh(image) =>
                    on_refresh(self, &mut updated, image),
                ResetFocus =>
                    on_reset_focus(self),
                ResetImage =>
                    on_reset_image(self, &mut updated),
                ResetScrolls(to_end) =>
                    on_reset_scrolls(self, to_end),
                Save(ref path, ref sources) =>
                    on_save(self, path, sources),
                SearchText(text, backward, color) =>
                    on_search_text(self, &mut updated, text, backward, color),
                SetEnv(name, value) =>
                    on_set_env(self, &name, &value.map(|it| it.to_string())),
                Scroll(direction, scroll_size, crush, reset_at_end, ref operation, reset_scrolls_1) =>
                    on_scroll(self, direction, scroll_size, crush, reset_at_end, operation, reset_scrolls_1, context),
                Shell(async, read_operations, search_path, as_binary, ref command_line, ref stdin_sources) =>
                    on_shell(self, async, read_operations, search_path, as_binary, command_line, stdin_sources),
                ShellFilter(ref command_line, search_path) =>
                    on_shell_filter(self, command_line, search_path),
                Show(count, ignore_views, move_by, _) =>
                    on_show(self, &mut updated, count, ignore_views, move_by),
                ShowCommandLine(ref initial) =>
                    on_show_command_line(self, initial),
                Shuffle(fix_current) =>
                    on_shuffle(self, &mut updated, fix_current),
                Sort(fix_current, sort_key, reverse) =>
                    on_sort(self, &mut updated, fix_current, sort_key, reverse),
                Sorter(fix_current, ref command, reverse) =>
                    on_sorter(self, &mut updated, fix_current, command, reverse),
                TellRegion(left, top, right, bottom, button) =>
                    on_tell_region(self, left, top, right, bottom, &button),
                Timer(name, op, interval, repeat, async) =>
                    on_timer(self, name, op, interval, repeat, async),
                UIAction(action_type) =>
                    on_ui_action(self, action_type),
                Unclip =>
                    on_unclip(self, &mut updated),
                Undo(count) =>
                    on_undo(self, &mut updated, count),
                Unmap(target) =>
                    on_unmap(self, &target),
                Unmark(target) =>
                    on_unmark(self, &target),
                Update(new_updated) =>
                    ok!(updated = new_updated),
                UpdateUI =>
                    panic!("WTF"),
                UpdateOption(ref option_name, ref updater) =>
                    on_update_option(self, &mut updated, option_name, updater),
                User(ref data) =>
                    on_user(self, data),
                Views(cols, rows, ignore_views) =>
                    on_views(self, &mut updated, cols, rows, ignore_views),
                ViewsFellow(for_rows, ignore_views) =>
                    on_views_fellow(self, &mut updated, for_rows, ignore_views),
                WakeupTimer(ref name) =>
                    on_wakeup_timer(self, name),
                When(filter, unless, op) =>
                    on_when(self, filter, unless, &op, context),
                WithMessage(message, op) =>
                    on_with_message(self, &mut updated, message, *op, context),
                Write(ref path, ref index) =>
                    on_write(self, path, index),
            };
            if let Err(err) = operated {
                puts_error!(err, "operation" => operation_name);
                return;
            }
        }

        updated.counter |= count != self.counter.peek();

        if self.states.spawned {
            self.after_operate(&mut updated, len, to_end);
        }
    }

    fn after_operate(&mut self, updated: &mut Updated, len: usize, to_end: bool) {
        if self.entries.len() != len {
            let gui_len = self.gui.len();
            if let Some(index) = self.paginator.current_index() {
                if index < len && len < index + gui_len {
                    updated.image = true;
                } else if self.states.auto_paging == AutoPaging::Always {
                    self.operate(Operation::Last(None, false, MoveBy::Page, false), None);
                    return
                } else if self.states.auto_paging == AutoPaging::Smart && gui_len <= len && len - gui_len == index {
                    self.operate(Operation::Next(None, false, MoveBy::Page, false, false), None);
                    return
                }
            }
        }

        if updated.counter {
            updated.label = true;
            self.update_counter_env(false);
        }

        if updated.pointer {
            self.send_lazy_draw(None, to_end);
            if !updated.message {
                self.update_message(None, false);
            }
            if self.paginator.at_last() {
                self.fire_event(&EventName::AtLast);
            } else if self.paginator.at_first() {
                self.fire_event(&EventName::AtFirst);
            }
        }

        if updated.image_options {
            self.do_clear_cache = true;
            // FIXME Re-draw just after UI updated
            self.send_lazy_draw(Some(100), to_end);
            return;
        }

        if updated.image || updated.image_options || updated.size {
            self.fire_event(&EventName::ShowImagePre);
            let (showed, original_image_size, fit_image_size) = time!("show_image" => self.show_image(to_end, updated.target_regions.clone()));
            self.on_image_updated(original_image_size, fit_image_size);
            self.update_watcher();
            if showed {
                self.fire_event(&EventName::ShowImage);
            }
        }

        if updated.image || updated.image_options || updated.label || updated.message || updated.remote | updated.size {
            self.update_label(updated.image, false);
        }
    }

    pub fn current(&self) -> Option<(Arc<Entry>, usize)> {
        self.current_with(0)
    }

    pub fn current_with(&self, delta: usize) -> Option<(Arc<Entry>, usize)> {
        self.paginator.current_index_with(delta).and_then(|index| {
            self.entries.nth(index).map(|it| (it, index))
        })
    }

    pub fn current_for_file(&self) -> Option<(PathBuf, usize, Arc<Entry>)> {
        self.current().and_then(|(entry, index)| {
            match entry.content {
                EntryContent::Image(ref path) => Some((path.clone(), index, Arc::clone(&entry))),
                _ => None
            }
        })
    }

    /**
     * @return (entry, index of entry, number of non fly leave pages)
     */
    pub fn current_non_fly_leave(&self) -> Option<(Arc<Entry>, usize, usize)> {
        let len = self.gui.len();
        for delta in 0..len {
            if let Some((entry, index)) = self.current_with(delta) {
                return Some((entry, index, len - delta));
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.current_non_fly_leave().is_none()
    }

    pub fn update_env_for_option(&self, option_name: &PreDefinedOptionName) {
        use session::{generate_option_value, WriteContext};
        use constant::OPTION_VARIABLE_PREFIX;

        let (name, value) = generate_option_value(option_name, &self.states, WriteContext::ENV);
        env::set_var(format!("{}{}", OPTION_VARIABLE_PREFIX, name), value.unwrap_or_else(||  o!("")));
    }

    pub fn app_info(&self) -> AppInfo {
        AppInfo {
            active: self.gui.window.is_active(),
            pages: self.entries.len(),
            real_pages: self.entries.real_len(),
            current_page: self.current().map(|it| it.1 + 1),
        }
    }

    pub fn paging(&mut self, wrap: bool, ignore_sight: bool) -> Paging {
        Paging {
            count: self.counter.take(),
            wrap,
            ignore_sight,
        }
    }

    pub fn paging_with_count(&mut self, wrap: bool, ignore_sight: bool, count: Option<usize>) -> Paging {
        Paging {
            count: self.counter.overwrite(count).take(),
            wrap,
            ignore_sight,
        }
    }

    pub fn paging_with_index(&mut self, wrap: bool, ignore_sight: bool, index: usize) -> Paging {
        self.paging_with_count(wrap, ignore_sight, Some(index + 1))
    }

    /* Private methods */

    fn get_imaging(&self) -> Imaging {
        let cell_size = self.gui.get_cell_size(&self.states.view);
        Imaging::new(cell_size, &self.states.drawing)
    }
    fn get_cell_size(&self) -> Size {
        self.gui.get_cell_size(&self.states.view)
    }

    fn reset_view(&mut self) {
        self.gui.reset_view(&self.states.view);
        self.update_paginator_condition();
    }

    fn send_lazy_draw(&mut self, delay: Option<u64>, to_end: bool) {
        self.draw_serial += 1;
        let op = Operation::LazyDraw(self.draw_serial, to_end);

        trace!("send_lazy_draw: delay={:?}", delay);

        if let Some(delay) = delay {
            let tx = self.tx.clone();
            spawn(move || {
                sleep(Duration::from_millis(delay));
                tx.send(op).unwrap();
            });
        } else {
            self.tx.send(op).unwrap();
        }
    }

    fn cancel_lazy_draw(&mut self) {
        self.draw_serial += 1;
    }

    fn do_go(&mut self, updated: &mut Updated) {
        let index = self.states.go.as_ref().and_then(|key| self.entries.search(key));
        if let Some(index) = index {
            if self.paginator.update_index(Index(index)) {
                updated.pointer = true;
                self.states.go = None;
            }
        }
    }

    fn pre_fetch(&mut self, cell_size: Size, range: Range<usize>) {
        let len = self.gui.len();
        let mut entries = VecDeque::new();

        for n in range {
            for index in 0..len {
                let index = index + len * n;
                if let Some((entry, _)) = self.current_with(index) {
                    entries.push_back(entry);
                }
            }
        }

        self.fetcher.new_target(entries, cell_size, self.states.drawing.clone());
    }

    /**
     * @return (Original size, Fit size)
     */
    fn show_image(&mut self, to_end: bool, target_regions: Option<Vec<Option<Region>>>) -> (bool, Option<Size>, Option<Size>) {
        let mut original_image_size = None;
        let mut fit_image_size = None;
        let cell_size = self.get_cell_size();

        self.cancel_lazy_draw();

        if self.states.pre_fetch.enabled {
            self.pre_fetch(cell_size, 0..1);
        }

        let mut invalid_all = true;
        let mut showed = false;
        let mut base_scale = None;

        for (index, cell) in self.gui.cells(self.states.reverse).enumerate() {
            if let Some((entry, _)) = self.current_with(index) {
                let image_buffer = self.cache.get_image_buffer(&entry, &Imaging::new(cell_size, &self.states.drawing));
                let imaging = self.get_imaging();
                match image_buffer {
                    Ok(image_buffer) => {
                        let scale = cell.draw(&image_buffer, imaging.cell_size, &self.states.drawing.fit_to);
                        if base_scale.is_none() {
                            base_scale = scale;
                        }
                        invalid_all = false;
                        if index == 0 {
                            original_image_size = image_buffer.get_original_size();
                            fit_image_size = image_buffer.get_fit_size();
                        }
                    }
                    Err(error) =>
                        cell.show_error(&error, imaging.cell_size)
                }
                showed = true;
            } else {
                cell.hide();
            }
        }

        self.current_base_scale = base_scale;

        if self.states.drawing.fit_to.is_scrollable() {
            self.tx.send(Operation::UpdateUI).unwrap();
            let op = if let Some(target_regions) = target_regions {
                Operation::MakeVisibles(target_regions)
            } else {
                Operation::ResetScrolls(to_end)
            };
            self.tx.send(op).unwrap();
        }

        if self.states.pre_fetch.enabled {
            self.pre_fetch_serial += 1;
            let pre_fetch_serial = self.pre_fetch_serial;
            let tx = self.tx.clone();
            spawn(move || {
                sleep(Duration::from_millis(200));
                tx.send(Operation::PreFetch(pre_fetch_serial)).unwrap();
            });
        }

        if showed && invalid_all {
            self.fire_event(&EventName::InvalidAll);
        }

        (showed, original_image_size, fit_image_size)
    }

    fn update_counter_env(&mut self, do_pop: bool) {
        let count = if do_pop {
            self.counter.take()
        } else {
            self.counter.peek()
        };
        env::set_var(constant::env_name("COUNT"), s!(count));
    }

    fn update_env(&mut self, envs: &[(String, String)]) {
        let mut new_keys = HashSet::<String>::new();
        for &(ref name, ref value) in envs {
            env::set_var(constant::env_name(name), value);
            new_keys.insert(o!(name));
        }
        for name in self.current_env_keys.difference(&new_keys) {
            env::remove_var(constant::env_name(name));
        }
        self.current_env_keys = new_keys;
    }

    /* Returns true if message is updated */
    fn update_message(&mut self, message: Option<String>, keep: bool) -> bool {
        if !self.states.spawned {
            return false;
        }

        if keep && self.last_message.is_some() {
            return false;
        }

        let name = constant::env_name("MESSAGE");
        if let Some(ref message) = message {
            env::set_var(name, message);
        } else {
            env::remove_var(name);
        }

        self.last_message = message;
        true
    }

    fn on_image_updated(&mut self, original_image_size: Option<Size>, fit_image_size: Option<Size>) {
        use entry::EntryContent::*;

        let mut envs: Vec<(String, String)> = vec![];
        let mut envs_sub: Vec<(String, String)> = vec![];
        let gui_len = self.gui.len();
        let len = self.entries.len();

        if let Some((entry, index, pages)) = self.current_non_fly_leave() {
            envs_sub.push((o!("path"), entry.display_path()));
            envs_sub.push((o!("abbrev_path"), entry.abbrev_path(self.states.abbrev_length)));
            envs_sub.push((o!("base_name"), entry.abbrev_path(0)));

            if let Some(ref meta) = entry.meta {
                for entry in meta.iter() {
                    envs.push((format!("meta_{}", entry.key), entry.value.clone()));
                }
            }

            // Path means local file path, url, or pdf file path
            match entry.content {
                Image(ref path) => {
                    envs.push((o!("type"), o!("image")));
                    envs.push((o!("file"), o!(path_to_str(path))));
                }
                Archive(ref archive_file, ref entry) => {
                    envs.push((o!("type"), o!("archive")));
                    envs.push((o!("file"), entry.name.clone()));
                    envs.push((o!("archive_file"), o!(path_to_str(&**archive_file))));
                    envs.push((o!("archive_page"), s!(entry.index + 1)));
                },
                Memory(_, ref hash) => {
                    envs.push((o!("type"), o!("memory")));
                    envs.push((o!("hash"), o!(hash)));
                },
                Pdf(ref pdf_file, index) => {
                    envs.push((o!("type"), o!("pdf")));
                    envs.push((o!("file"), o!(path_to_str(&**pdf_file))));
                    envs.push((o!("archive_page"), s!(index + 1)));
                }
            }

            if let Some(ref url) = entry.url {
                envs.push((o!("url"), o!(**url)));
                if let Some(path) = entry.content.local_file_path() {
                    envs.push((o!("cache_path"), o!(path_to_str(&path))));
                }
            }

            envs.push((o!("page"), s!(index + 1)));
            envs.push((o!("begin_page"), s!(index + 1)));
            envs.push((o!("end_page"), s!(index + pages)));
            envs.push((o!("pages"), s!(len)));

            if let Some(size) = original_image_size {
                envs.push((o!("width"), s!(size.width)));
                envs.push((o!("height"), s!(size.height)));
                let (w, h) = size.ratio();
                envs.push((o!("ratio"), format!("{}:{}", w, h)));
            }

            if let Some(size) = fit_image_size.or(original_image_size) {
                envs_sub.push((o!("fit_width"), s!(size.width)));
                envs_sub.push((o!("fit_height"), s!(size.height)));
                let (w, h) = size.ratio();
                envs_sub.push((o!("fit_ratio"), format!("{}:{}", w, h)));
            }

            if let (Some(original), Some(fit)) = (original_image_size, fit_image_size) {
                envs_sub.push((o!("scale"), format!("{}", fit.width as f32 / original.width as f32)));
                envs_sub.push((o!("scale_pct"), format!("{}", fit.width * 100 / original.width)));
            }

            envs_sub.push((o!("paging"), {
                let (from, to) = (index + 1, min!(index + pages, len));
                if gui_len > 1 {
                    if self.states.reverse {
                        format!("{}←{}", to, from)
                    } else {
                        format!("{}→{}", from, to)
                    }
                } else {
                    format!("{}", from)
                }
            }));

            envs_sub.push((o!("flags"), {
                use self::FitTo::*;
                let mut text = o!("");
                text.push(match self.states.drawing.fit_to {
                    Cell => 'C',
                    Height => 'H',
                    Original => 'O',
                    OriginalOrCell => 'o',
                    Width => 'W',
                    Fixed(_, _) => 'F',
                    Scale(_) => 'S',
                });
                text.push(if self.states.reverse { 'R' } else { 'r' });
                text.push(if self.states.auto_paging.enabled() { 'A' } else { 'a' });
                text.push(if self.states.auto_reload { 'W' } else { 'w' });
                text
            }));
        }

        {
            let cell_size = self.gui.get_cell_size(&self.states.view);
            envs_sub.push((o!("cell_width"), s!(cell_size.width)));
            envs_sub.push((o!("cell_height"), s!(cell_size.height)));
        }

        puts_show_event(&envs);
        envs.extend_from_slice(&envs_sub);
        self.update_env(&envs);
    }

    fn update_style(&self) {
        if let Err(err) = self.gui.update_style(&self.states.style) {
            puts_error!(err, "at" => "gui/update_style");
        }
    }

    fn update_label(&self, update_title: bool, force_empty: bool) {
        env::set_var(constant::env_name("pages"), s!(self.entries.len()));

        let empty = force_empty || self.is_empty();

        if update_title {
            let text =
                if empty {
                    o!(constant::DEFAULT_INFORMATION)
                } else {
                    self.states.title_format.generate()
                };
            self.gui.window.set_title(&text);
        }

        let text =
            if empty {
                self.states.empty_status_format.generate()
            } else {
                self.states.status_format.generate()
            };
        self.gui.set_status_bar_markup(&text);
    }

    fn update_ui_visibility(&mut self) {
        self.gui.set_status_bar_visibility(self.states.status_bar);

        match self.gui.change_screen(self.states.screen, &self.tx) {
            Ok(changed) if !changed => return,
            Ok(_) => (),
            Err(err) => {
                puts_error!(err, "at" => "gui/update_ui_visibility");
                return
            },
        }

        match self.states.screen {
            Screen::LogView => {
                let mut buffer = self.log.buffer.lock().unwrap();
                self.gui.append_logs(&buffer);
                buffer.clear();
            },
            Screen::CommandLine => {
                let user_operations = self.mapping.operation_mapping.operations();
                self.gui.update_user_operations(&user_operations);
            },
            _ => (),
        }
    }

    fn update_status_bar_height(&self) {
        self.gui.set_status_bar_height(self.states.status_bar_height);
    }

    fn update_watcher(&self) {
        if !self.states.auto_reload && !self.states.watch_files {
            self.watcher.clear();
            return;
        }

        let len = self.gui.len();
        let mut targets = HashSet::new();

        for delta in 0..len {
            if let Some((entry, _)) = self.current_with(delta) {
                if let EntryContent::Image(ref path) = entry.content {
                    targets.insert(path.to_path_buf());
                }
            }
        }

        self.watcher.update(targets);
    }

    fn initialize_envs_for_options(&self) {
        for option_name in PreDefinedOptionName::iterator() {
            self.update_env_for_option(option_name)
        }
    }

    fn update_paginator_condition(&mut self) {
        let condition = paginator::Condition {
            len: self.entries.len(),
            sight_size: self.states.view.len()
        };
        self.paginator.update_condition(&condition);
    }

    fn store(&self) -> Option<Serial> {
        self.current().map(|it| it.0.serial)
    }

    fn restore_or_first(&mut self, updated: &mut Updated, serial: Option<Serial>, ignore_views: bool) {
        updated.pointer = if let Some(index) = serial.and_then(|it| self.entries.search_by_serial(it)) {
            if ignore_views {
                let paging = self.paging_with_count(false, true, Some(index + 1));
                self.paginator.first(&paging)
            } else {
                self.paginator.update_index(Index(index))
            }
        } else {
            let paging = self.paging(false, false);
            self.paginator.first(&paging)
        }
    }
}


fn puts_show_event(envs: &[(String, String)]) {
    let mut pairs = vec![(o!("event"), o!("show"))];
    pairs.extend_from_slice(envs);
    logger::puts(&pairs);
}

fn set_envs() {
    unsafe {
        let pid = s!(libc::getpid());
        env::set_var(constant::env_name("PID"), &pid);
        puts_event!("info/pid", "value" => pid);
    }

    let version = env!("CARGO_PKG_VERSION").to_string();
    let sha = env!("VERGEN_SHA");
    let date = env!("VERGEN_COMMIT_DATE");

    puts_event!("version", "version" => version, "git_hash" => sha, "date" => date);

    env::set_var(constant::env_name("GIT_HASH"), sha);
    env::set_var(constant::env_name("GIT_DATE"), date);
    env::set_var(constant::env_name("VERSION"), version);
}
