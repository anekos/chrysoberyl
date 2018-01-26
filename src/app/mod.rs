
use std::collections::{HashSet, HashMap, VecDeque};
use std::env;
use std::ops::Range;
use std::path::PathBuf;
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
use controller;
use counter::Counter;
use entry::{Entry, EntryContainer, EntryContent, Serial};
use error;
use events::EventName;
use gui::Gui;
use image_cache::ImageCache;
use image_fetcher::ImageFetcher;
use logger;
use mapping::{Mapping, Input};
use operation::option::PreDefinedOptionName;
use operation::{Operation, QueuedOperation, OperationContext, MappingTarget, MoveBy, Updated};
use option::user_switch::UserSwitchManager;
use paginator::values::Index;
use paginator::{self, Paginator, Paging};
use remote_cache::RemoteCache;
use script;
use size::{Size, FitTo, Region};
use sorting_buffer::SortingBuffer;
use state::{States, PreFetchState};
use termination;
use timer::TimerManager;
use ui_event;
use util::path::path_to_str;
use version;
use watcher::Watcher;

mod error_loop_detector;
mod on_events;
pub mod info;

use self::info::AppInfo;



pub struct App {
    counter: Counter,
    current_base_scale: Option<f64>, // Scale of first scaled image
    current_env_keys: HashSet<String>,
    do_clear_cache: bool,
    draw_serial: u64,
    encodings: Vec<EncodingRef>,
    error_loop_detector: error_loop_detector::Detector,
    fetcher: ImageFetcher,
    found_on: Option<Range<usize>>,
    pre_fetch_serial: u64,
    remote_cache: RemoteCache,
    rng: ThreadRng,
    search_text: Option<String>,
    sorting_buffer: SortingBuffer<QueuedOperation>,
    timers: TimerManager,
    user_switches: UserSwitchManager,
    watcher: Watcher,
    pub cache: ImageCache,
    pub mapping: Mapping,
    pub entries: EntryContainer,
    pub gui: Gui,
    pub paginator: Paginator,
    pub primary_tx: Sender<Operation>,
    pub states: States,
    pub tx: Sender<Operation>,
}


impl App {
    pub fn new(mut initial: Initial) -> (App, Receiver<Operation>, Receiver<Operation>) {
        let (tx, rx) = channel();
        let (primary_tx, primary_rx) = channel();

        let mut states = States::default();

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
            counter: Counter::new(),
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
            mapping: Mapping::new(),
            paginator: Paginator::new(),
            pre_fetch_serial: 0,
            primary_tx: primary_tx,
            remote_cache: RemoteCache::new(initial.curl_threads, tx.clone(), sorting_buffer.clone()),
            rng: rand::thread_rng(),
            search_text: None,
            sorting_buffer: sorting_buffer,
            states: states,
            timers: TimerManager::new(tx.clone()),
            tx: tx.clone(),
            user_switches: UserSwitchManager::new(tx.clone()),
            watcher: Watcher::new(tx.clone()),
        };

        script::load(&app.tx, &config::get_config_source(), &app.states.path_list);
        app.tx.send(Operation::InitialProcess(initial.entries, initial.shuffle, initial.stdin_as_file)).unwrap();
        error::register(app.tx.clone());

        (app, primary_rx, rx)
    }

    pub fn fire_event_with_context(&mut self, event_name: &EventName, context: HashMap<String, String>) {
        use self::EventName::*;

        let op = event_name.operation_with_context(context);

        match *event_name {
            Initialize => self.tx.send(op).unwrap(),
            _ => self.operate(op),
        }
    }

    pub fn fire_event(&mut self, event_name: &EventName) {
        self.fire_event_with_context(event_name, HashMap::new())
    }

    pub fn operate(&mut self, operation: Operation) {
        self.operate_with_context(operation, None)
    }

    pub fn operate_with_context(&mut self, operation: Operation, context: Option<OperationContext>) {
        use self::Operation::*;
        use self::on_events::*;

        let operation_name = d!(operation);
        trace!("operate_with_context: operation={:?}", operation_name);

        let mut updated = Updated::default();
        let mut to_end = false;
        let len = self.entries.len();

        {
            let operated = match operation {
                AppEvent(ref event_name, ref context) =>
                    on_app_event(self, &mut updated, event_name, context),
                Cherenkov(ref parameter) =>
                    on_cherenkov(self, &mut updated, parameter, context),
                Clear =>
                    on_clear(self, &mut updated),
                Clip(region) =>
                    on_clip(self, &mut updated, region, context),
                Context(context, op) =>
                    return self.operate_with_context(*op, Some(context)),
                Count(count) =>
                    ok!(self.counter.set(count)),
                CountDigit(digit) =>
                    ok!(self.counter.push_digit(digit)),
                DefineUserSwitch(name, values) =>
                    on_define_switch(self, name, values),
                Delete(expr) =>
                    on_delete(self, &mut updated, *expr),
                Draw =>
                    ok!(updated.image = true),
                Editor(ref editor_command, ref files, ref sessions) =>
                   on_editor(self, editor_command.clone(), files, sessions),
                Error(error) =>
                    on_error(self, &mut updated, error),
                Expand(recursive, ref base) =>
                    on_expand(self, &mut updated, recursive, base.clone()),
                FileChanged(ref path) =>
                    on_file_changed(self, &mut updated, path),
                Fill(shape, region, color, mask, cell_index) =>
                    on_fill(self, &mut updated, shape, region, color, mask, cell_index, context),
                Filter(dynamic, expr) =>
                    on_filter(self, &mut updated, dynamic, *expr),
                First(count, ignore_views, move_by, _) =>
                    on_first(self, &mut updated, count, ignore_views, move_by),
                FlyLeaves(n) =>
                    on_fly_leaves(self, &mut updated, n),
                Fragile(ref path) =>
                    on_fragile(self, path),
                Go(ref key) =>
                    on_go(self, &mut updated, key),
                InitialProcess(entries, shuffle, stdin_as_file) =>
                    on_initial_process(self, entries, shuffle, stdin_as_file),
                Input(ref input) =>
                    on_input(self, input),
                KillTimer(ref name) =>
                    on_kill_timer(self, name),
                Last(count, ignore_views, move_by, _) =>
                    on_last(self, &mut updated, count, ignore_views, move_by),
                LazyDraw(serial, new_to_end) =>
                    on_lazy_draw(self, &mut updated, &mut to_end, serial, new_to_end),
                Load(ref file, search_path) =>
                    on_load(self, file, search_path),
                LoadDefault =>
                    on_load_default(self),
                MakeVisibles(ref regions) =>
                    on_make_visibles(self, regions),
                Map(target, remain, mapped_operation) =>
                    on_map(self, target, remain, mapped_operation),
                Meow =>
                    on_meow(self, &mut updated),
                MoveAgain(count, ignore_views, move_by, wrap) =>
                    on_move_again(self, &mut updated, &mut to_end, count, ignore_views, move_by, wrap),
                Multi(ops, async) =>
                    on_multi(self, ops, async),
                Next(count, ignore_views, move_by, wrap) =>
                    on_next(self, &mut updated, count, ignore_views, move_by, wrap),
                Nop =>
                    Ok(()),
                OperateFile(ref file_operation) =>
                    on_operate_file(self, file_operation),
                Page(page) =>
                    on_page(self, &mut updated, page),
                PdfIndex(async, read_operations, search_path, ref command_line, ref fmt, ref separator) =>
                    on_pdf_index(self, async, read_operations, search_path, command_line, fmt, separator.as_ref().map(String::as_str)),
                PreFetch(pre_fetch_serial) =>
                    on_pre_fetch(self, pre_fetch_serial),
                Previous(count, ignore_views, move_by, wrap) =>
                    on_previous(self, &mut updated, &mut to_end, count, ignore_views, move_by, wrap),
                Pull =>
                    on_pull(self, &mut updated),
                Push(path, meta, force) =>
                    on_push(self, &mut updated, path.to_string(), meta, force),
                PushArchive(file, meta, force) =>
                    on_push_archive(self, &file.expand(), meta, force, None),
                PushDirectory(file, meta, force) =>
                    on_push_directory(self, &mut updated, file.expand(), meta, force),
                PushImage(file, meta, force, expand_level) =>
                    on_push_image(self, &mut updated, file.expand(), meta, force, expand_level, None),
                PushMemory(buf) =>
                    on_push_memory(self, &mut updated, buf),
                PushPdf(file, meta, force) =>
                    on_push_pdf(self, &mut updated, file.expand(), meta, force, None),
                PushSibling(next, meta, force, show) =>
                    on_push_sibling(self, &mut updated, next, meta, force, show),
                PushURL(url, meta, force, entry_type) =>
                    on_push_url(self, &mut updated, url, meta, force, entry_type),
                Random =>
                    on_random(self, &mut updated, len),
                Refresh(image) =>
                    on_refresh(self, &mut updated, image),
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
                Scroll(ref direction, ref operation, scroll_size, crush) =>
                    on_scroll(self, direction, operation, scroll_size, crush),
                Shell(async, read_operations, search_path, ref command_line, ref stdin_sources) =>
                    on_shell(self, async, read_operations, search_path, command_line, stdin_sources),
                ShellFilter(ref command_line, search_path) =>
                    on_shell_filter(self, command_line, search_path),
                Show(count, ignore_views, move_by, _) =>
                    on_show(self, &mut updated, count, ignore_views, move_by),
                Shuffle(fix_current) =>
                    on_shuffle(self, &mut updated, fix_current),
                Sort(fix_current, sort_key, reverse) =>
                    on_sort(self, &mut updated, fix_current, sort_key, reverse),
                TellRegion(left, top, right, bottom, button) =>
                    on_tell_region(self, left, top, right, bottom, &button),
                Timer(name, op, interval, repeat) =>
                    on_timer(self, name, op, interval, repeat),
                Unclip =>
                    on_unclip(self, &mut updated),
                Undo(count) =>
                    on_undo(self, &mut updated, count),
                Unmap(target) =>
                    on_unmap(self, &target),
                Update(new_updated) =>
                    ok!(updated = new_updated),
                UpdateUI =>
                    panic!("WTF"),
                UpdateOption(ref option_name, ref updater) =>
                    on_update_option(self, &mut updated, option_name, updater),
                User(ref data) =>
                    on_user(self, data),
                Views(cols, rows) =>
                    on_views(self, &mut updated, cols, rows),
                ViewsFellow(for_rows) =>
                    on_views_fellow(self, &mut updated, for_rows),
                When(filter, unless, op) =>
                    on_when(self, filter, unless, &op),
                WithMessage(message, op) =>
                    on_with_message(self, &mut updated, message, *op),
                Write(ref path, ref index) =>
                    on_write(self, path, index),
            };
            if let Err(err) = operated {
                puts_error!(err, "operation" => operation_name);
                return;
            }
        }

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
                } else if self.states.auto_paging && gui_len <= len && len - gui_len == index {
                    self.operate(Operation::Next(None, false, MoveBy::Page, false));
                    return
                }
            }
        }

        if updated.pointer {
            self.send_lazy_draw(None, to_end);
            if !updated.message {
                self.update_message(None);
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

        if updated.image || updated.image_options {
            let image_size = time!("show_image" => self.show_image(to_end, updated.target_regions.clone()));
            self.on_image_updated(image_size);
            self.update_watcher();
        }

        if updated.image || updated.image_options || updated.label || updated.message {
            self.update_label(updated.image, false);
        }
    }

    pub fn current(&self) -> Option<(Entry, usize)> {
        self.current_with(0)
    }

    pub fn current_with(&self, delta: usize) -> Option<(Entry, usize)> {
        self.paginator.current_index_with(delta).and_then(|index| {
            self.entries.nth(index).map(|it| (it, index))
        })
    }

    pub fn current_for_file(&self) -> Option<(PathBuf, usize, Entry)> {
        self.current().and_then(|(entry, index)| {
            match entry.content {
                EntryContent::Image(ref path) => Some((path.clone(), index, entry.clone())),
                _ => None
            }
        })
    }

    /**
     * @return (entry, index of entry, number of non fly leave pages)
     */
    pub fn current_non_fly_leave(&self) -> Option<(Entry, usize, usize)> {
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

        let (name, value) = generate_option_value(option_name, &self.states, &self.gui, WriteContext::ENV);
        env::set_var(format!("{}{}", OPTION_VARIABLE_PREFIX, name), value.unwrap_or_else(||  o!("")));
    }

    pub fn app_info(&self) -> AppInfo {
        AppInfo {
            pages: self.entries.len(),
            real_pages: self.entries.real_len(),
            current_page: self.current().map(|it| it.1 + 1),
        }
    }

    pub fn paging(&mut self, wrap: bool, ignore_sight: bool) -> Paging {
        Paging {
            count: self.counter.pop(),
            wrap: wrap,
            ignore_sight: ignore_sight,
        }
    }

    pub fn paging_with_count(&mut self, wrap: bool, ignore_sight: bool, count: Option<usize>) -> Paging {
        Paging {
            count: self.counter.overwrite(count).pop(),
            wrap: wrap,
            ignore_sight: ignore_sight,
        }
    }

    pub fn paging_with_index(&mut self, wrap: bool, ignore_sight: bool, index: usize) -> Paging {
        self.paging_with_count(wrap, ignore_sight, Some(index + 1))
    }

    /* Private methods */

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

    fn show_image(&mut self, to_end: bool, target_regions: Option<Vec<Option<Region>>>) -> Option<Size> {
        let mut image_size = None;
        let cell_size = self.gui.get_cell_size(&self.states.view, self.states.status_bar);

        self.cancel_lazy_draw();

        if self.states.pre_fetch.enabled {
            self.pre_fetch(cell_size, 0..1);
        }

        let mut invalid_all = true;
        let mut showed = false;
        let mut base_scale = None;

        for (index, cell) in self.gui.cells(self.states.reverse).enumerate() {
            if let Some((entry, _)) = self.current_with(index) {
                let image_buffer = self.cache.get_image_buffer(&entry, &cell_size, &self.states.drawing);
                let (fg, bg) = (self.gui.colors.error, self.gui.colors.error_background);
                match image_buffer {
                    Ok(image_buffer) => {
                        let scale = cell.draw(&image_buffer, &cell_size, &self.states.drawing.fit_to, &fg, &bg);
                        if base_scale.is_none() {
                            base_scale = scale;
                        }
                        invalid_all = false;
                        if index == 0 {
                            image_size = image_buffer.get_original_size();
                        }
                    }
                    Err(error) =>
                        cell.draw_text(&error, &cell_size, &fg, &bg)
                }
                showed = true;
            } else {
                cell.image.set_from_pixbuf(None);
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

        if showed {
            trace!("showed");
            self.fire_event(&EventName::ShowImage);
            if invalid_all {
                self.fire_event(&EventName::InvalidAll);
            }
        }

        image_size
    }

    fn update_env(&mut self, envs: &[(String, String)]) {
        let mut new_keys = HashSet::<String>::new();
        for &(ref name, ref value) in envs {
            env::set_var(constant::env_name(name), value);
            new_keys.insert(o!(name));
        }
        for name in self.current_env_keys.difference(&new_keys) {
            env::remove_var(name);
        }
        self.current_env_keys = new_keys;
    }

    fn update_message(&self, message: Option<String>) {
        if !self.states.spawned {
            return;
        }

        let name = constant::env_name("MESSAGE");

        if let Some(message) = message {
            env::set_var(name, message);
        } else {
            env::remove_var(name);
        }
    }

    fn on_image_updated(&mut self, image_size: Option<Size>) {
        use entry::EntryContent::*;

        let mut envs: Vec<(String, String)> = vec![];
        let mut envs_sub: Vec<(String, String)> = vec![];
        let gui_len = self.gui.len();
        let len = self.entries.len();

        if let Some((entry, index, pages)) = self.current_non_fly_leave() {
            envs_sub.push((o!("path"), entry.display_path()));
            envs_sub.push((o!("abbrev_path"), entry.abbrev_path(self.states.abbrev_length)));
            envs_sub.push((o!("base_name"), entry.abbrev_path(0)));

            if let Some(meta) = entry.meta {
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

            if let Some(url) = entry.url {
                envs.push((o!("url"), o!(*url)));
                if let Some(path) = entry.content.local_file_path() {
                    envs.push((o!("cache_path"), o!(path_to_str(&path))));
                }
            }

            envs.push((o!("page"), s!(index + 1)));
            envs.push((o!("begin_page"), s!(index + 1)));
            envs.push((o!("end_page"), s!(index + pages)));
            envs.push((o!("pages"), s!(len)));

            if let Some(image_size) = image_size {
                envs.push((o!("width"), s!(image_size.width)));
                envs.push((o!("height"), s!(image_size.height)));
                let (w, h) = image_size.ratio();
                envs.push((o!("ratio"), format!("{}:{}", w, h)));
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
                text.push(if self.states.auto_paging { 'A' } else { 'a' });
                text.push(if self.states.view.center_alignment { 'C' } else { 'c' });
                text.push(if self.states.auto_reload { 'W' } else { 'w' });
                text
            }));
        }

        puts_show_event(&envs);
        envs.extend_from_slice(&envs_sub);
        self.update_env(&envs);
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

        if self.states.status_bar {
            let text =
                if empty {
                    self.states.empty_status_format.generate()
                } else {
                    self.states.status_format.generate()
                };
            self.gui.label.set_markup(&text);
        }
    }

    fn update_label_visibility(&self) {
        if self.states.status_bar {
            self.gui.label.show();
        } else {
            self.gui.label.hide();
        }
    }

    fn update_watcher(&self) {
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

    fn restore_or_first(&mut self, updated: &mut Updated, serial: Option<Serial>) {
        updated.pointer = if let Some(index) = serial.and_then(|it| self.entries.search_by_serial(it)) {
            self.paginator.update_index(Index(index))
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
    let sha = version::sha();
    let date = version::commit_date();

    puts_event!("version", "version" => version, "git_hash" => sha, "date" => date);

    env::set_var(constant::env_name("GIT_HASH"), sha);
    env::set_var(constant::env_name("GIT_DATE"), date);
    env::set_var(constant::env_name("VERSION"), version);
}
