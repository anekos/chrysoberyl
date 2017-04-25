
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::spawn;
use std::collections::HashSet;

use encoding::types::EncodingRef;
use gdk_pixbuf::PixbufAnimation;
use gtk::Image;
use gtk::prelude::*;
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};
use libc;
use rand::distributions::{IndependentSample, Range};
use rand::{self, ThreadRng};

use archive::{self, ArchiveEntry};
use cherenkov::Cherenkoved;
use color::Color;
use config;
use constant;
use controller;
use editor;
use entry::{Entry, EntryContent, EntryContainer, EntryContainerOptions, MetaSlice, new_meta};
use events;
use filer;
use fragile_input::new_fragile_input;
use gui::{Gui, ColorTarget, Direction};
use http_cache::HttpCache;
use image_buffer;
use index_pointer::IndexPointer;
use mapping::{Mapping, Input};
use operation::{self, Operation, OperationContext, MappingTarget};
use option::{OptionValue, OptionUpdateMethod};
use output;
use shell;
use size::FitTo;
use state::ScalingMethod;
use state::{States, StateName};
use termination;
use utils::path_to_str;



pub struct App {
    entries: EntryContainer,
    cherenkoved: Cherenkoved,
    mapping: Mapping,
    http_cache: HttpCache,
    encodings: Vec<EncodingRef>,
    gui: Gui,
    draw_serial: u64,
    rng: ThreadRng,
    pointer: IndexPointer,
    current_env_keys: HashSet<String>,
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

struct Updated {
    pointer: bool,
    label: bool,
    image: bool
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

        let mut app = App {
            entries: EntryContainer::new(entry_options),
            cherenkoved: Cherenkoved::new(),
            gui: gui.clone(),
            tx: tx.clone(),
            http_cache: HttpCache::new(initial.http_threads, tx.clone()),
            states: states,
            encodings: initial.encodings,
            mapping: Mapping::new(),
            draw_serial: 0,
            rng: rand::thread_rng(),
            pointer: IndexPointer::new(),
            current_env_keys: HashSet::new(),
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

        for file in &initial.files {
           app.on_push(file.clone(), &[]);
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

    pub fn operate(&mut self, operation: &Operation) {
        self.operate_with_context(operation, None)
    }

    pub fn operate_with_context(&mut self, operation: &Operation, context: Option<&OperationContext>) {
        use self::Operation::*;

        let mut updated = Updated { pointer: false, label: false, image: false };
        let mut to_end = false;
        let len = self.entries.len();

        debug!("Operate\t{:?}", operation);

        {
            match *operation {
                ChangeFitTo(ref fit) =>
                    self.on_change_fit_to(&mut updated, fit),
                ChangeScalingMethod(ref scaling_method) =>
                    self.on_change_scaling_method(&mut updated, scaling_method),
                Cherenkov(ref parameter) =>
                    self.on_cherenkov(&mut updated, parameter, context),
                CherenkovClear =>
                    self.on_cherenkov_clear(&mut updated),
                Clear =>
                    self.on_clear(&mut updated),
                Color(ref target, ref color) =>
                    self.on_color(&mut updated, target, color),
                Context(ref context, ref op) =>
                    return self.operate_with_context(op, Some(context)),
                Count(count) =>
                    self.pointer.set_count(count),
                CountDigit(digit) =>
                    self.pointer.push_count_digit(digit),
                Editor(ref editor_command, ref config_sources) =>
                   self.on_editor(editor_command.clone(), config_sources.to_owned()),
                Expand(recursive, ref base) =>
                    self.on_expand(&mut updated, recursive, base),
                First(count, ignore_views) =>
                    updated.pointer = self.pointer.with_count(count).first(len, !ignore_views),
                ForceFlush =>
                    self.http_cache.force_flush(),
                Fragile(ref path) =>
                    self.on_fragile(path),
                Initialized =>
                    self.on_initialized(),
                Input(ref input) =>
                    self.on_input(input),
                Last(count, ignore_views) =>
                    updated.pointer = self.pointer.with_count(count).last(len, !ignore_views),
                LazyDraw(serial, new_to_end) =>
                    self.on_lazy_draw(&mut updated, &mut to_end, serial, new_to_end),
                LoadConfig(ref config_source) =>
                    self.on_load_config(config_source),
                Map(ref target, ref mapped_operation) =>
                    self.on_map(target, mapped_operation),
                Multi(ref ops) =>
                    self.on_multi(ops),
                Next(count, ignore_views) =>
                    updated.pointer = self.pointer.with_count(count).next(len, !ignore_views),
                Nop =>
                    (),
                OperateFile(ref file_operation) =>
                    self.on_operate_file(file_operation),
                Previous(count, ignore_views) =>
                    self.on_previous(&mut updated, &mut to_end, count, ignore_views),
                PrintEntries =>
                    self.on_print_entries(),
                Push(ref path, ref meta) =>
                    self.on_push(path.clone(), meta),
                PushArchiveEntry(ref archive_path, ref entry) =>
                    self.on_push_archive_entry(&mut updated, archive_path, entry),
                PushHttpCache(ref file, ref url, ref meta) =>
                    self.on_push_http_cache(&mut updated, file, url, meta),
                PushFile(ref file, ref meta) =>
                    self.on_push_path(&mut updated, file.clone(), meta),
                PushPdf(ref file, ref meta) =>
                    self.on_push_pdf(&mut updated, file.clone(), meta),
                PushURL(ref url, ref meta) =>
                    self.on_push_url(url.clone(), meta),
                Quit =>
                    termination::execute(),
                Random =>
                    self.on_random(&mut updated, len),
                Refresh =>
                    updated.pointer = true,
                Save(ref path, ref index) =>
                    self.on_save(path, index),
                Scroll(ref direction, ref operation, scroll_size) =>
                    self.on_scroll(direction, operation, scroll_size),
                Shell(async, read_operations, ref command_line) =>
                    shell::call(async, command_line, option!(read_operations, self.tx.clone())),
                Shuffle(fix_current) =>
                    self.on_shuffle(&mut updated, fix_current),
                Sort =>
                    self.on_sort(&mut updated),
                UpdateOption(ref name, ref modifier) =>
                    self.on_update_option(&mut updated, name, modifier),
                User(ref data) =>
                    self.on_user(data),
                Views(cols, rows) =>
                    self.on_views(&mut updated, cols, rows),
                ViewsFellow(for_rows) =>
                    self.on_views_fellow(&mut updated, for_rows),
            }
        }

        if self.states.initialized && self.entries.len() != len {
            if let Some(current) = self.pointer.current {
                let gui_len = self.gui.len();
                if current < len && len < current + gui_len {
                    updated.image = true;
                } else if self.states.auto_paging.is_enabled() && gui_len <= len && len - gui_len == current {
                    self.operate(&Operation::Next(None, false));
                    return
                }
            }
        }

        if updated.pointer {
            self.draw_serial += 1;
            self.tx.send(Operation::LazyDraw(self.draw_serial, to_end)).unwrap();
        }

        if updated.image && self.states.initialized {
            time!("show_image" => self.show_image(to_end));
            self.puts_event_with_current("show", None);
        }

        if updated.image || updated.label {
            self.update_label();
            self.update_env();
        }
    }

    fn reset_view(&mut self) {
        self.gui.reset_view(&self.states.view);
    }

    /* Operation event */

    fn on_change_fit_to(&mut self, updated: &mut Updated, fit: &FitTo) {
        self.states.fit_to = fit.clone();
        updated.image = true;
    }

    fn on_change_scaling_method(&mut self, updated: &mut Updated, method: &ScalingMethod) {
        self.states.scaling = method.clone();
        updated.image = true;
    }

    fn on_cherenkov(&mut self, updated: &mut Updated, parameter: &operation::CherenkovParameter, context: Option<&OperationContext>) {
        use cherenkov::Che;
        use gdk_pixbuf::PixbufAnimationExt;

        fn get_image_size(image: &Image) -> Option<(i32, i32)> {
            image.get_pixbuf()
                .map(|it| (it.get_width(), it.get_height()))
                .or_else(|| {
                    image.get_animation()
                        .map(|it| (it.get_width(), it.get_height()))
                })
        }

        if let Some(OperationContext::Input(Input::MouseButton((mx, my), _))) = context.cloned() {
            let cell_size = self.gui.get_cell_size(&self.states.view, self.states.status_bar.is_enabled());

            for (index, cell) in self.gui.cells(self.states.reverse.is_enabled()).enumerate() {
                if let Some(entry) = self.entries.current_with(&self.pointer, index).map(|(entry,_)| entry) {
                    let (x1, y1, w, h) = {
                        let (cx, cy, cw, ch) = cell.get_top_left();
                        if let Some((iw, ih)) = get_image_size(&cell.image) {
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
                        self.cherenkoved.cherenkov(
                            &entry,
                            &cell_size,
                            &self.states.fit_to,
                            &Che {
                                center: center,
                                n_spokes: parameter.n_spokes,
                                radius: parameter.radius,
                                random_hue: parameter.random_hue,
                                color: parameter.color,
                            },
                            &self.states.scaling);
                        updated.image = true;
                    }
                }
            }
        }
    }

    fn on_cherenkov_clear(&mut self, updated: &mut Updated) {
        if let Some(entry) = self.entries.current_entry(&self.pointer) {
            self.cherenkoved.remove(&entry);
            updated.image = true;
        }
    }

    fn on_clear(&mut self, updated: &mut Updated) {
        self.entries.clear(&mut self.pointer);
        updated.image = true;
    }

    fn on_color(&mut self, updated: &mut Updated, target: &ColorTarget, color: &Color) {
        use self::ColorTarget::*;

        self.gui.update_color(target, color);

        updated.image = match *target {
            Error | ErrorBackground => true,
            _ => false
        };
    }

    fn on_editor(&mut self, editor_command: Option<String>, config_sources: Vec<config::ConfigSource>) {
        let tx = self.tx.clone();
        spawn(move || editor::start_edit(&tx, editor_command, config_sources));
    }

    fn on_expand(&mut self, updated: &mut Updated, recursive: bool, base: &Option<PathBuf>) {
        let count = self.pointer.counted();
        if recursive {
            self.entries.expand(&mut self.pointer, base.clone(), 1, count as u8);
        } else {
            self.entries.expand(&mut self.pointer, base.clone(), count as u8, count as u8- 1);
        }
        updated.label = true;
    }

    fn on_fragile(&mut self, path: &PathBuf) {
        new_fragile_input(self.tx.clone(), path_to_str(path));
    }

    fn on_initialized(&mut self) {
        self.states.initialized = true;
        puts_event!("initialized");
    }

    fn on_input(&mut self, input: &Input) {
        let (width, height) = self.gui.window.get_size();
        if let Some(op) = self.mapping.matched(input, width, height) {
            match op {
                Ok(op) =>
                    self.operate(&Operation::Context(OperationContext::Input(input.clone()), Box::new(op))),
                Err(err) =>
                    puts_error!("at" => "input", "reason" => err)
            }
        } else {
            self.puts_event_with_current(
                input.type_name(),
                Some(&[(o!("name"), o!(input.text()))]));
        }
    }

    fn on_lazy_draw(&mut self, updated: &mut Updated, to_end: &mut bool, serial: u64, new_to_end: bool) {
        trace!("draw_serial: {}, serial: {}", self.draw_serial, serial);
        if self.draw_serial == serial {
            updated.image = true;
            *to_end = new_to_end;
        }
    }

    fn on_load_config(&mut self, config_source: &config::ConfigSource) {
        config::load_config(&self.tx, config_source);
    }

    fn on_map(&mut self, target: &MappingTarget, operation: &[String]) {
        use self::MappingTarget::*;

        // FIXME
        puts_event!("map",
                    "target" => format!("{:?}", target),
                    "operation" => format!("{:?}", operation));
        match *target {
            Key(ref key) =>
                self.mapping.register_key(key, operation),
            Mouse(ref button, ref area) =>
                self.mapping.register_mouse(*button, area.clone(), operation)
        }
    }

    fn on_multi(&mut self, operations: &[Operation]) {
        for op in operations {
            self.operate(op)
        }
    }

    fn on_operate_file(&mut self, file_operation: &filer::FileOperation) {
        use entry::EntryContent::*;

        if let Some((entry, _)) = self.entries.current(&self.pointer) {
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

    fn on_previous(&mut self, updated: &mut Updated, to_end: &mut bool, count: Option<usize>, ignore_views: bool) {
        updated.pointer = self.pointer.with_count(count).previous(!ignore_views);
        *to_end = count.is_none() && !ignore_views;
    }

    fn on_print_entries(&self) {
        use std::io::{Write, stderr};
        for entry in self.entries.to_displays() {
            writeln!(&mut stderr(), "{}", entry).unwrap();
        }
    }

    fn on_push(&mut self, path: String, meta: &MetaSlice) {
        if path.starts_with("http://") || path.starts_with("https://") {
            self.tx.send(Operation::PushURL(path, new_meta(meta))).unwrap();
            return;
        }

        if let Ok(path) = Path::new(&path).canonicalize() {
            if let Some(ext) = path.extension() {
                match &*ext.to_str().unwrap().to_lowercase() {
                    "zip" | "rar" | "tar.gz" | "lzh" | "lha" =>
                        return archive::fetch_entries(&path, &self.encodings, self.tx.clone()),
                    "pdf" =>
                        return self.tx.send(Operation::PushPdf(path.clone(), new_meta(meta))).unwrap(),
                    _ => ()
                }
            }
        }

        self.operate(&Operation::PushFile(Path::new(&path).to_path_buf(), new_meta(meta)));
    }

    fn on_push_archive_entry(&mut self, updated: &mut Updated, archive_path: &PathBuf, entry: &ArchiveEntry) {
        updated.pointer = self.entries.push_archive_entry(&mut self.pointer, archive_path, entry);
        updated.label = true;
    }

    fn on_push_http_cache(&mut self, updated: &mut Updated, file: &PathBuf, url: &str, meta: &MetaSlice) {
        updated.pointer = self.entries.push_http_cache(&mut self.pointer, file, url, meta);
        updated.label = true;
    }

    fn on_push_path(&mut self, updated: &mut Updated, file: PathBuf, meta: &MetaSlice) {
        updated.pointer = self.entries.push_path(&mut self.pointer, &file, meta);
        updated.label = true;
    }

    fn on_push_pdf(&mut self, updated: &mut Updated, file: PathBuf, meta: &MetaSlice) {
        use poppler::PopplerDocument;
        let doc = PopplerDocument::new_from_file(&file);
        updated.pointer = self.entries.push_pdf(&mut self.pointer, &file, doc, meta);
        updated.label = true;
    }

    fn on_push_url(&mut self, url: String, meta: &MetaSlice) {
        self.http_cache.fetch(url, meta);
    }

    fn on_random(&mut self, updated: &mut Updated, len: usize) {
        if len > 0 {
            self.pointer.current = Some(Range::new(0, len).ind_sample(&mut self.rng));
            updated.image = true;
        }
    }

    fn on_save(&mut self, path: &PathBuf, index: &Option<usize>) {
        let count = index.unwrap_or_else(|| self.pointer.counted()) - 1;
        if let Err(error) = self.gui.save(path, count) {
            puts_error!("at" => "save", "reason" => error)
        }
    }

    fn on_scroll(&mut self, direction: &Direction, operation: &[String], scroll_size: f64) {
        if !self.gui.scroll_views(direction, scroll_size, self.pointer.counted()) && !operation.is_empty() {
            match Operation::parse_from_vec(operation) {
                Ok(op) => self.operate(&op),
                Err(err) => puts_error!("at" => "scroll", "reason" => err),
            }
        }
    }

    fn on_shuffle(&mut self, updated: &mut Updated, fix_current: bool) {
        self.entries.shuffle(&mut self.pointer, fix_current);
        if !fix_current {
            updated.image = true;
        }
        updated.label = true;
    }

    fn on_sort(&mut self, updated: &mut Updated) {
        self.entries.sort(&mut self.pointer);
        updated.label = true;
    }

    fn on_update_option(&mut self, updated: &mut Updated, name: &StateName, method: &OptionUpdateMethod) {
        use state::StateName::*;

        match *name {
            StatusBar => self.states.status_bar.update(method),
            Reverse => self.states.reverse.update(method),
            CenterAlignment => self.states.view.center_alignment.update(method),
            AutoPaging => self.states.auto_paging.update(method),
            FitTo => self.states.fit_to.update(method),
        };

        match *name {
            StatusBar => self.update_label_visibility(),
            CenterAlignment => self.reset_view(),
            _ => ()
        }

        updated.image = true;
    }

    fn on_user(&self, data: &[(String, String)]) {
        self.puts_event_with_current("user", Some(data));
    }

    fn on_views(&mut self, updated: &mut Updated, cols: Option<usize>, rows: Option<usize>) {
        if let Some(cols) = cols {
            self.states.view.cols = cols
        }
        if let Some(rows) = rows {
            self.states.view.rows = rows
        }
        updated.image = true;
        self.reset_view();
        self.pointer.set_multiplier(self.gui.len());
    }

    fn on_views_fellow(&mut self, updated: &mut Updated, for_rows: bool) {
        let count = self.pointer.counted();
        if for_rows {
            self.states.view.rows = count;
        } else {
            self.states.view.cols = count;
        };
        updated.image = true;
        self.reset_view();
        self.pointer.set_multiplier(self.gui.len());
    }

    /* Private methods */

    fn get_meta(&self, entry: &Entry) -> Option<Result<GenericMetadata, immeta::Error>> {
        use self::EntryContent::*;

        match (*entry).content {
            File(ref path) | Http(ref path, _) =>
                Some(immeta::load_from_file(&path)),
            Archive(_, ref entry) =>
                Some(immeta::load_from_buf(&entry.content)),
            Pdf(_, _, _) =>
                None
        }
    }

    fn puts_event_with_current(&self, event: &str, data: Option<&[(String, String)]>) {
        let mut pairs = vec![(o!("event"), o!(event))];
        pairs.extend_from_slice(self.current_env().as_slice());
        if let Some(data) = data {
            pairs.extend_from_slice(data);
        }
        output::puts(&pairs);
    }

    fn get_animation(&self, entry: &Entry) -> Option<Result<PixbufAnimation, image_buffer::Error>> {
        self.get_meta(entry).and_then(|img| {
            if let Ok(img) = img {
                if let Ok(gif) = img.into::<Gif>() {
                    if gif.is_animated() {
                        return Some(image_buffer::get_pixbuf_animation(entry));
                    }
                }
            }
            None
        })
    }

    fn show_image(&mut self, to_end: bool) {
        let cell_size = self.gui.get_cell_size(&self.states.view, self.states.status_bar.is_enabled());

        if self.states.fit_to.is_scrollable() {
            self.gui.reset_scrolls(to_end);
        }

        for (index, cell) in self.gui.cells(self.states.reverse.is_enabled()).enumerate() {
            if let Some(entry) = self.entries.current_with(&self.pointer, index).map(|(entry,_)| entry) {
                match self.get_animation(&entry) {
                    Some(pixbuf) => match pixbuf {
                        Ok(animation) => cell.draw_animation(&animation),
                        Err(error) => cell.draw_error(&error, &cell_size, &self.states.fit_to, &self.gui.colors)
                    },
                    None =>
                        match self.cherenkoved.get_pixbuf(&entry, &cell_size, &self.states.fit_to, &self.states.scaling) {
                            Ok(pixbuf) => cell.draw(&pixbuf, &cell_size, &self.states.fit_to),
                            Err(error) => cell.draw_error(&error, &cell_size, &self.states.fit_to, &self.gui.colors)
                        }
                }
            } else {
                cell.image.set_from_pixbuf(None);
            }
        }
    }

    fn update_env(&mut self) {
        let mut new_keys = HashSet::<String>::new();
        for (name, value) in self.current_env() {
            env::set_var(constant::env_name(&name), value);
            new_keys.insert(name);
        }
        for name in self.current_env_keys.difference(&new_keys) {
            env::remove_var(name);
        }
        self.current_env_keys = new_keys;
    }

    fn current_env(&self) -> Vec<(String, String)> {
        use entry::EntryContent::*;

        let mut envs: Vec<(String, String)> = vec![];

        if let Some((entry, index)) = self.entries.current(&self.pointer) {
            for entry in entry.meta.iter() {
                envs.push((format!("meta_{}", entry.key), entry.value.clone()));
            }
            match entry.content {
                File(ref path) => {
                    envs.push((o!("file"), o!(path_to_str(path))));
                }
                Http(ref path, ref url) => {
                    envs.push((o!("file"), o!(path_to_str(path))));
                    envs.push((o!("url"), o!(url)));
                }
                Archive(ref archive_file, ref entry) => {
                    envs.push((o!("file"), entry.name.clone()));
                    envs.push((o!("archive_file"), o!(path_to_str(archive_file))));
                },
                Pdf(ref pdf_file, _, index) => {
                    envs.push((o!("file"), o!(path_to_str(pdf_file))));
                    envs.push((o!("pdf_page"), s!(index)));
                }
            }
            let last_page = min!(index + self.gui.len(), self.entries.len());
            envs.push((o!("page"), s!(index + 1)));
            envs.push((o!("last_page"), s!(last_page)));
            envs.push((o!("count"), s!(self.entries.len())));
        }

        envs
    }

    fn update_label(&self) {
        let text =
            if let Some((entry, index)) = self.entries.current(&self.pointer) {
                let len = self.entries.len();
                let gui_len = self.gui.len();
                let (from, to) = (index + 1, min!(index + gui_len, len));
                let mut text =
                    if gui_len > 1 {
                        if self.states.reverse.is_enabled() {
                            format!("[{}←{}/{}]", to, from, len)
                        } else {
                            format!("[{}→{}/{}]", from, to, len)
                        }
                    } else {
                        format!("[{}/{}]", from, len)
                    };
                text.push(' ');
                text.push_str(&entry.display_path());
                text.push_str(" {");
                text.push(self.states.fit_to.to_char());
                text.push(self.states.reverse.to_char());
                text.push(self.states.auto_paging.to_char());
                text.push(self.states.view.center_alignment.to_char());
                text.push('}');
                text
            } else {
                o!(constant::DEFAULT_INFORMATION)
            };

        self.gui.window.set_title(&text);
        if self.states.status_bar.is_enabled() {
            self.gui.label.set_text(&text);
        }
    }

    fn update_label_visibility(&self) {
        if self.states.status_bar.is_enabled() {
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
