
use std::collections::HashSet;
use std::collections::VecDeque;
use std::env;
use std::ops::Range;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::{sleep, spawn};
use std::time::Duration;

use encoding::types::EncodingRef;
use gtk::prelude::*;
use libc;
use rand::{self, ThreadRng};

use constant;
use controller;
use entry::{EntryContainer, EntryContainerOptions};
use events;
use gui::Gui;
use http_cache::HttpCache;
use image_cache::ImageCache;
use image_fetcher::ImageFetcher;
use index_pointer::IndexPointer;
use mapping::{Mapping, Input};
use operation::{Operation, QueuedOperation, OperationContext, MappingTarget, MoveBy};
use option::user::UserSwitchManager;
use output;
use shellexpand_wrapper as sh;
use size::{Size, FitTo, Region};
use sorting_buffer::SortingBuffer;
use state::{States, PreFetchState};
use termination;
use timer::TimerManager;
use utils::path_to_str;


mod on_events;


pub struct App {
    entries: EntryContainer,
    mapping: Mapping,
    http_cache: HttpCache,
    encodings: Vec<EncodingRef>,
    gui: Gui,
    draw_serial: u64,
    pre_fetch_serial: u64,
    rng: ThreadRng,
    pointer: IndexPointer,
    current_env_keys: HashSet<String>,
    cache: ImageCache,
    fetcher: ImageFetcher,
    sorting_buffer: SortingBuffer<QueuedOperation>,
    timers: TimerManager,
    user_switches: UserSwitchManager,
    pub tx: Sender<Operation>,
    pub states: States
}

pub struct Initial {
    pub http_threads: u8,
    pub expand: bool,
    pub expand_recursive: bool,
    pub shuffle: bool,
    pub controllers: controller::Controllers,
    pub files: Vec<String>,
    pub encodings: Vec<EncodingRef>,
    pub operations: Vec<String>
}

#[derive(Default)]
pub struct Updated {
    pointer: bool,
    label: bool,
    image: bool,
    image_options: bool,
}


impl App {
    pub fn new(initial: Initial, states: States, gui: Gui, entry_options:EntryContainerOptions) -> (App, Receiver<Operation>, Receiver<Operation>) {
        let (tx, rx) = channel();
        let (primary_tx, primary_rx) = channel();

        let mut initial = initial;

        if initial.encodings.is_empty() {
            use encoding::all::*;
            initial.encodings.push(UTF_8);
            initial.encodings.push(WINDOWS_31J);
        }

        unsafe {
            let pid = s!(libc::getpid());
            env::set_var(&constant::env_name("PID"), &pid);
            puts_event!("info/pid", "value" => pid);
        }

        let cache_limit = PreFetchState::default().limit_of_items;
        let cache = ImageCache::new(cache_limit);

        let sorting_buffer = SortingBuffer::new();

        let mut app = App {
            entries: EntryContainer::new(entry_options),
            gui: gui.clone(),
            tx: tx.clone(),
            http_cache: HttpCache::new(initial.http_threads, tx.clone(), sorting_buffer.clone()),
            states: states,
            encodings: initial.encodings,
            mapping: Mapping::new(),
            draw_serial: 0,
            pre_fetch_serial: 0,
            rng: rand::thread_rng(),
            pointer: IndexPointer::new(),
            current_env_keys: HashSet::new(),
            cache: cache.clone(),
            fetcher: ImageFetcher::new(cache),
            sorting_buffer: sorting_buffer,
            timers: TimerManager::new(tx.clone()),
            user_switches: UserSwitchManager::new(tx.clone()),
        };

        app.reset_view();

        for op in &initial.operations {
            match Operation::from_str(op) {
                Ok(op) => tx.send(op).unwrap(),
                Err(err) => puts_error!("at" => "operation", "reason" => s!(err)),
            }
        }

        events::register(&gui, &primary_tx);
        controller::register(&tx, &initial.controllers);

        app.update_label_visibility();

        {
            let mut updated = Updated::default();
            for file in &initial.files {
                on_events::on_push(&mut app, &mut updated, file.clone(), None, false);
            }
        }

        {
            let mut expand_base = None;

            if app.entries.len() == 0 {
                if let Some(file) = initial.files.get(0) {
                    expand_base = Path::new(file).to_path_buf().parent().map(|it| it.to_path_buf());
                }
            }

            if initial.expand {
                tx.send(Operation::Expand(false, expand_base)).unwrap();
            } else if initial.expand_recursive {
                tx.send(Operation::Expand(true, expand_base)).unwrap();
            }
        }

        if initial.shuffle {
            let fix = initial.files.get(0).map(|it| Path::new(it).is_file()).unwrap_or(false);
            tx.send(Operation::Shuffle(fix)).unwrap();
        }

        (app, primary_rx, rx)
    }

    pub fn operate(&mut self, operation: Operation) {
        self.operate_with_context(operation, None)
    }

    pub fn operate_with_context(&mut self, operation: Operation, context: Option<OperationContext>) {
        use self::Operation::*;
        use self::on_events::*;

        let mut updated = Updated { pointer: false, label: false, image: false, image_options: false };
        let mut to_end = false;
        let len = self.entries.len();

        {
            match operation {
                Cherenkov(ref parameter) =>
                    on_cherenkov(self, &mut updated, parameter, &context),
                CherenkovClear =>
                    on_cherenkov_clear(self, &mut updated),
                Clear =>
                    on_clear(self, &mut updated),
                Clip(region) =>
                    on_clip(self, &mut updated, region),
                Context(context, op) =>
                    return self.operate_with_context(*op, Some(context)),
                Count(count) =>
                    self.pointer.set_count(count),
                CountDigit(digit) =>
                    self.pointer.push_count_digit(digit),
                DefineUserSwitch(name, values) =>
                    on_define_switch(self, name, values),
                Draw =>
                    updated.image = true,
                Editor(ref editor_command, ref config_sources) =>
                   on_editor(self, editor_command.clone(), config_sources.to_owned()),
                Expand(recursive, ref base) =>
                    on_expand(self, &mut updated, recursive, base.clone()),
                Filter(command_line) =>
                    on_filter(self, command_line),
                Fill(region, cell_index) =>
                    on_fill(self, &mut updated, region, cell_index),
                First(count, ignore_views, move_by, _) =>
                    on_first(self, &mut updated, len, count, ignore_views, move_by),
                Fragile(ref path) =>
                    on_fragile(self, path),
                Initialized =>
                    return on_initialized(self),
                Input(ref input) =>
                    on_input(self, input),
                KillTimer(ref name) =>
                    on_kill_timer(self, name),
                Last(count, ignore_views, move_by, _) =>
                    on_last(self, &mut updated, len, count, ignore_views, move_by),
                LazyDraw(serial, new_to_end) =>
                    on_lazy_draw(self, &mut updated, &mut to_end, serial, new_to_end),
                Load(ref script_source) =>
                    on_load(self, script_source),
                Map(ref target, ref mapped_operation) =>
                    on_map(self, target, mapped_operation.to_vec()),
                MoveEntry(ref from, ref to) =>
                    on_move_entry(self, &mut updated, from, to),
                Multi(ops, async) =>
                    on_multi(self, ops, async),
                Next(count, ignore_views, move_by, wrap) =>
                    on_next(self, &mut updated, len, count, ignore_views, move_by, wrap),
                Nop =>
                    (),
                OperateFile(ref file_operation) =>
                    on_operate_file(self, file_operation),
                PreFetch(pre_fetch_serial) =>
                    on_pre_fetch(self, pre_fetch_serial),
                Previous(count, ignore_views, move_by, wrap) =>
                    on_previous(self, &mut updated, len, &mut to_end, count, ignore_views, move_by, wrap),
                PrintEntries =>
                    on_print_entries(self),
                Pull =>
                    on_pull(self, &mut updated),
                Push(path, meta, force) =>
                    on_push(self, &mut updated, path, meta, force),
                PushImage(file, meta, force) =>
                    on_push_image(self, &mut updated, file, meta, force),
                PushPdf(file, meta, force) =>
                    on_push_pdf(self, &mut updated, file, meta, force),
                PushSibling(next, meta, force, show) =>
                    on_push_sibling(self, &mut updated, next, meta, force, show),
                PushURL(url, meta, force) =>
                    on_push_url(self, &mut updated, url, meta, force),
                Quit =>
                    termination::execute(),
                Random =>
                    on_random(self, &mut updated, len),
                Refresh =>
                    updated.pointer = true,
                Save(ref path, ref sources) =>
                    on_save(self, path, sources),
                SetEnv(ref name, ref value) =>
                    on_set_env(self, name, value),
                Scroll(ref direction, ref operation, scroll_size) =>
                    on_scroll(self, direction, operation, scroll_size),
                Shell(async, read_operations, ref command_line, ref stdin_sources) =>
                    on_shell(self, async, read_operations, command_line, self.tx.clone(), stdin_sources),
                Show(ref key) =>
                    on_show(self, &mut updated, key),
                Shuffle(fix_current) =>
                    on_shuffle(self, &mut updated, fix_current),
                Sort =>
                    on_sort(self, &mut updated),
                TellRegion(ref region) =>
                    on_tell_region(self, region),
                Timer(name, op, interval, repeat) =>
                    on_timer(self, name, op, interval, repeat),
                Unclip => 
                    on_unclip(self, &mut updated),
                UpdateOption(ref option_name, ref updater) =>
                    on_update_option(self, &mut updated, option_name, updater.clone()),
                User(ref data) =>
                    on_user(self, data),
                Views(cols, rows) =>
                    on_views(self, &mut updated, cols, rows),
                ViewsFellow(for_rows) =>
                    on_views_fellow(self, &mut updated, for_rows),
                Write(ref path, ref index) =>
                    on_write(self, path, index),
                WindowResized =>
                    on_window_resized(self, &mut updated),
            }
        }

        if !self.states.initialized {
            return
        }

        if self.entries.len() != len {
            if let Some(current) = self.pointer.current {
                let gui_len = self.gui.len();
                if current < len && len < current + gui_len {
                    updated.image = true;
                } else if self.states.auto_paging && gui_len <= len && len - gui_len == current {
                    self.operate(Operation::Next(None, false, MoveBy::Page, false));
                    return
                }
            }
        }

        if updated.pointer {
            self.send_lazy_draw(None, to_end);
        }

        if updated.image_options {
            puts_event!("pre_fetch_cache/clear");
            self.cache.clear();
            // FIXME Re-draw just after UI updated
            self.send_lazy_draw(Some(100), to_end);
            return;
        }

        if updated.image || updated.image_options {
            let image_size = time!("show_image" => self.show_image(to_end));
            self.on_image_updated(image_size);
        }

        if updated.image || updated.image_options || updated.label {
            self.update_label(updated.image);
        }
    }

    fn reset_view(&mut self) {
        self.gui.reset_view(&self.states.view);
    }

    fn send_lazy_draw(&mut self, delay: Option<u64>, to_end: bool) {
        self.draw_serial += 1;
        let op = Operation::LazyDraw(self.draw_serial, to_end);

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

    /* Private methods */

    fn do_show(&mut self, updated: &mut Updated) {
        let index = self.states.show.as_ref().and_then(|key| self.entries.search(key));
        if let Some(index) = index {
            self.pointer.current = Some(index);
            updated.pointer = true;
            self.states.show = None;
        }
    }

    fn pre_fetch(&mut self, cell_size: Size, range: Range<usize>) {
        let len = self.gui.len();
        let mut entries = VecDeque::new();

        for n in range {
            for index in 0..len {
                let index = index + len * n;
                if let Some(entry) = self.entries.current_with(&self.pointer, index).map(|(entry,_)| entry) {
                    entries.push_back(entry);
                }
            }
        }

        self.fetcher.new_target(entries, cell_size, self.states.drawing.clone());
    }

    fn show_image(&mut self, to_end: bool) -> Option<Size> {
        let image_size = None;
        let cell_size = self.gui.get_cell_size(&self.states.view, self.states.status_bar);

        if self.states.drawing.fit_to.is_scrollable() {
            self.gui.reset_scrolls(to_end);
        }

        if self.states.pre_fetch.enabled {
            self.pre_fetch(cell_size, 0..1);
        }

        for (index, cell) in self.gui.cells(self.states.reverse).enumerate() {
            if let Some(entry) = self.entries.current_with(&self.pointer, index).map(|(entry,_)| entry) {
                let image_buffer = self.cache.get_image_buffer(&entry, &cell_size, &self.states.drawing);
                let (fg, bg) = (self.gui.colors.error, self.gui.colors.error_background);
                match image_buffer {
                    Ok(image_buffer) =>
                        cell.draw(&image_buffer, &cell_size, &self.states.drawing.fit_to, &fg, &bg),
                    Err(error) =>
                        cell.draw_text(&error, &cell_size, &fg, &bg)
                }
            } else {
                cell.image.set_from_pixbuf(None);
            }
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

    fn on_image_updated(&mut self, image_size: Option<Size>) {
        use entry::EntryContent::*;

        let mut envs: Vec<(String, String)> = vec![];
        let mut envs_sub: Vec<(String, String)> = vec![];
        let len = self.entries.len();
        let gui_len = self.gui.len();

        if let Some((entry, index)) = self.entries.current(&self.pointer) {
            if let Some(meta) = entry.meta {
                for entry in meta.iter() {
                    envs.push((format!("meta_{}", entry.key), entry.value.clone()));
                }
            }

            // Path means local file path, url, or pdf file path
            match entry.content {
                File(ref path) => {
                    envs.push((o!("file"), o!(path_to_str(path))));
                    envs_sub.push((o!("path"), o!(path_to_str(path))));
                }
                Http(ref path, ref url) => {
                    envs.push((o!("file"), o!(path_to_str(path))));
                    envs.push((o!("url"), o!(url)));
                    envs_sub.push((o!("path"), o!(url)))
                }
                Archive(ref archive_file, ref entry) => {
                    envs.push((o!("file"), entry.name.clone()));
                    envs.push((o!("archive_file"), o!(path_to_str(archive_file))));
                    envs_sub.push((o!("path"), entry.name.clone()));
                },
                Pdf(ref pdf_file, index) => {
                    envs.push((o!("file"), o!(path_to_str(pdf_file))));
                    envs.push((o!("pdf_page"), s!(index)));
                    envs_sub.push((o!("path"), o!(path_to_str(pdf_file))));
                }
            }

            let last_page = min!(index + gui_len, len);
            envs.push((o!("page"), s!(index + 1)));
            envs.push((o!("last_page"), s!(last_page)));
            envs.push((o!("count"), s!(self.entries.len())));

            if let Some(image_size) = image_size {
                envs.push((o!("width"), s!(image_size.width)));
                envs.push((o!("height"), s!(image_size.height)));
            }

            envs_sub.push((o!("paging"), {
                let (from, to) = (index + 1, min!(index + gui_len, len));
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
                });
                text.push(if self.states.reverse { 'R' } else { 'r' });
                text.push(if self.states.auto_paging { 'A' } else { 'a' });
                text.push(if self.states.view.center_alignment { 'C' } else { 'c' });
                text
            }));
        }

        puts_show_event(&envs);
        envs.extend_from_slice(&envs_sub);
        self.update_env(&envs);
    }

    fn update_label(&self, update_title: bool) {
        env::set_var(constant::env_name("count"), s!(self.entries.len()));


        if update_title {
            let text =
                if self.entries.current(&self.pointer).is_some() {
                    sh::expand(&self.states.title_format.0)
                } else {
                    o!(constant::DEFAULT_INFORMATION)
                };
            self.gui.window.set_title(&text);
        }

        if self.states.status_bar {
            let text =
                if self.entries.current(&self.pointer).is_some() {
                    sh::expand(&self.states.status_format.0)
                } else {
                    o!(constant::DEFAULT_INFORMATION)
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
}


impl Initial {
    pub fn new() -> Initial {
        Initial {
            http_threads: 3,
            expand: false,
            expand_recursive: false,
            shuffle: false,
            files: vec![],
            controllers: controller::Controllers::new(),
            encodings: vec![],
            operations: vec![],
        }
    }
}


fn puts_show_event(envs: &[(String, String)]) {
    let mut pairs = vec![(o!("event"), o!("show"))];
    pairs.extend_from_slice(envs);
    output::puts(&pairs);
}
