
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::spawn;
use std::collections::{HashMap,HashSet};

use css_color_parser::Color;
use encoding::types::EncodingRef;
use gtk::prelude::*;
use gtk::Image;
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};
use rand::{self, ThreadRng};
use rand::distributions::{IndependentSample, Range};

use archive::{self, ArchiveEntry};
use cherenkov::Cherenkoved;
use config;
use constant;
use controller;
use editor;
use entry::{Entry, EntryContainer, EntryContainerOptions};
use events;
use filer;
use fragile_input::new_fragile_input;
use gui::{Gui, ColorTarget};
use http_cache::HttpCache;
use image_buffer;
use index_pointer::IndexPointer;
use mapping::{Mapping, Input};
use operation::{self, Operation, StateUpdater, OperationContext, MappingTarget};
use output;
use shell;
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
                Err(err) => puts_error!("at" => "operation", "reason" => err),
            }
        }

        for fragile in initial.controllers.fragiles.clone() {
            new_fragile_input(&fragile);
        }

        events::register(gui, primary_tx.clone());
        controller::register(tx.clone(), &initial.controllers);

        app.update_label_visibility();

        for file in &initial.files {
           app.on_push(file.clone());
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
        let len = self.entries.len();

        debug!("Operate\t{:?}", operation);

        {
            match *operation {
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
                First(count) =>
                    updated.pointer = self.pointer.with_count(count).first(len),
                Input(ref input) =>
                    self.on_input(input),
                Last(count) =>
                    updated.pointer = self.pointer.with_count(count).last(len),
                LazyDraw(serial) =>
                    self.on_lazy_draw(&mut updated, serial),
                LoadConfig(ref config_source) =>
                    self.on_load_config(config_source),
                Map(ref target, ref mapped_operation) =>
                    self.on_map(target, mapped_operation),
                Multi(ref ops) =>
                    self.on_multi(ops),
                Next(count) =>
                    self.on_next(&mut updated, count, len),
                Nop =>
                    (),
                OperateFile(ref file_operation) =>
                    self.on_operate_file(file_operation),
                Previous(count) =>
                    self.on_previous(&mut updated, count),
                PrintEntries =>
                    self.on_print_entries(),
                Push(ref path) =>
                    self.on_push(path.clone()),
                PushArchiveEntry(ref archive_path, ref entry) =>
                    self.on_push_archive_entry(&mut updated, archive_path, entry),
                PushHttpCache(ref file, ref url) =>
                    self.on_push_http_cache(&mut updated, file, url),
                PushPath(ref file) =>
                    self.on_push_path(&mut updated, file.clone()),
                PushURL(ref url) =>
                    self.on_push_url(url.clone()),
                Quit =>
                    termination::execute(),
                Random =>
                    self.on_random(&mut updated, len),
                Refresh =>
                    updated.pointer = true,
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

        if updated.pointer {
            self.draw_serial += 1;
            self.tx.send(Operation::LazyDraw(self.draw_serial)).unwrap();
        }
        if updated.image {
            time!("show_image" => self.show_image());
            self.puts_event_with_current("show", None);
        }

        if updated.image || updated.label {
            if let Some((entry, index)) = self.entries.current(&self.pointer) {
                let len = self.entries.len();
                let path = entry.display_path();
                self.update_label(&format!("[{}/{}] {}", index + 1, len, path));
            } else {
                self.gui.window.set_title(constant::DEFAULT_TITLE);
                self.update_label(constant::DEFAULT_INFORMATION);
            }
            self.update_env();
        }
    }

    fn reset_view(&mut self) {
        self.gui.reset_view(&self.states.view);
    }

    /* Operation event */

    fn on_cherenkov(&mut self, updated: &mut Updated, parameter: &operation::CherenkovParameter, context: Option<&OperationContext>) {
        use cherenkov::Che;
        use gtk::WidgetExt;
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
            let (cw, ch) = self.gui.get_cell_size(&self.states.view, self.states.status_bar);

            for (index, image) in self.gui.images(self.states.reverse).enumerate() {
                if let Some(entry) = self.entries.current_with(&self.pointer, index).map(|(entry,_)| entry) {
                    let (x1, y1, w, h) = {
                        let a = image.get_allocation();
                        if let Some((iw, ih)) = get_image_size(image) {
                        (a.x + (a.width - iw) / 2, a.y + (a.height - ih) / 2, iw, ih)
                        } else {
                            continue;
                        }
                    };
                    let (x2, y2) = (x1 + w, y1 + h);
                    if x1 <= mx && mx <= x2 && y1 <= my && my <= y2 {
                        let center = (
                            parameter.x.unwrap_or_else(|| mx - x1),
                            parameter.y.unwrap_or_else(|| my - y1));
                        self.cherenkoved.cherenkov(
                            &entry, cw, ch,
                            &Che {
                                center: center,
                                n_spokes: parameter.n_spokes,
                                radius: parameter.radius,
                                random_hue: parameter.random_hue,
                                color: parameter.color,
                            });
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
        spawn(|| editor::start_edit(tx, editor_command, config_sources));
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

    fn on_input(&mut self, input: &Input) {
        let (width, height) = self.gui.window.get_size();
        if let Some(op) = self.mapping.matched(input, width, height) {
            let op = Operation::Context(OperationContext::Input(input.clone()), Box::new(op));
            self.tx.send(op).unwrap();
        } else {
            self.puts_event_with_current(
                input.type_name(),
                Some(&[(o!("name"), o!(input.text()))]));
        }
    }

    fn on_lazy_draw(&mut self, updated: &mut Updated, serial: u64) {
        trace!("draw_serial: {}, serial: {}", self.draw_serial, serial);
        if self.draw_serial == serial {
            updated.image = true;
        }
    }

    fn on_load_config(&mut self, config_source: &config::ConfigSource) {
        config::load_config(self.tx.clone(), config_source);
    }

    fn on_map(&mut self, target: &MappingTarget, operation: &Box<Operation>) {
        use self::MappingTarget::*;

        // FIXME
        puts_event!("map",
                    "target" => format!("{:?}", target),
                    "operation" => format!("{:?}", operation));
        match *target {
            Key(ref key) =>
                self.mapping.register_key(key, *operation.clone()),
            Mouse(ref button, ref area) =>
                self.mapping.register_mouse(*button, area.clone(), *operation.clone())
        }
    }

    fn on_multi(&mut self, operations: &[Operation]) {
        for op in operations {
            self.operate(op)
        }
    }

    fn on_next(&mut self, updated: &mut Updated, count: Option<usize>, len: usize) {
        updated.pointer = self.pointer.with_count(count).next(len);
    }

    fn on_operate_file(&mut self, file_operation: &filer::FileOperation) {
        use entry::Entry::*;

        if let Some((entry, _)) = self.entries.current(&self.pointer) {
            let result = match entry {
                File(ref path) | Http(ref path, _) => file_operation.execute(path),
                Archive(_ , _) => Err(o!("copy/move does not support archive files."))
            };
            let text = format!("{:?}", file_operation);
            match result {
                Ok(_) => puts_event!("operate_file", "status" => "ok", "operation" => text),
                Err(err) => puts_event!("operate_file", "status" => "fail", "reason" => err, "operation" => text),
            }
        }
    }

    fn on_previous(&mut self, updated: &mut Updated,  count: Option<usize>) {
        updated.pointer = self.pointer.with_count(count).previous();
    }

    fn on_print_entries(&self) {
        use std::io::{Write, stderr};
        for entry in self.entries.to_displays() {
            writeln!(&mut stderr(), "{}", entry).unwrap();
        }
    }

    fn on_push(&mut self, path: String) {
        if path.starts_with("http://") || path.starts_with("https://") {
            self.tx.send(Operation::PushURL(path)).unwrap();
            return;
        }

        if let Ok(path) = Path::new(&path).canonicalize() {
            if let Some(ext) = path.extension() {
                match &*ext.to_str().unwrap().to_lowercase() {
                    "zip" | "rar" | "tar.gz" | "lzh" | "lha" =>
                        return archive::fetch_entries(&path, &self.encodings, self.tx.clone()),
                    _ => ()
                }
            }
        }

        self.operate(&Operation::PushPath(Path::new(&path).to_path_buf()));
    }

    fn on_push_archive_entry(&mut self, updated: &mut Updated, archive_path: &PathBuf, entry: &ArchiveEntry) {
        updated.pointer = self.entries.push_archive_entry(&mut self.pointer, archive_path, entry);
        updated.label = true;
    }

    fn on_push_http_cache(&mut self, updated: &mut Updated, file: &PathBuf, url: &str) {
        updated.pointer = self.entries.push_http_cache(&mut self.pointer, file, url);
        updated.label = true;
    }

    fn on_push_path(&mut self, updated: &mut Updated, file: PathBuf) {
        updated.pointer = self.entries.push_path(&mut self.pointer, &file);
        updated.label = true;
    }

    fn on_push_url(&mut self, url: String) {
        self.http_cache.fetch(url);
    }

    fn on_random(&mut self, updated: &mut Updated, len: usize) {
        if len > 0 {
            self.pointer.current = Some(Range::new(0, len).ind_sample(&mut self.rng));
            updated.image = true;
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

    fn on_update_option(&mut self, updated: &mut Updated, name: &StateName, modifier: &StateUpdater) {
        use state::StateName::*;
        use self::StateUpdater::*;

        {
            let value: &mut bool = match *name {
                StatusBar => &mut self.states.status_bar,
                Reverse => &mut self.states.reverse,
                CenterAlignment => &mut self.states.view.center_alignment,
            };

            match *modifier {
                Toggle => *value ^= true,
                Enable => *value = true,
                Disable => *value = false,
            }
        }

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
        self.pointer.multiply(self.gui.len());
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
        self.pointer.multiply(self.gui.len());
    }

    /* Private methods */

    fn current_info(&self) -> Vec<(String, String)> {
        use entry::Entry::*;
        use std::fmt::Display;

        fn push<K: Display, V: Display>(pairs: &mut Vec<(String, String)>, key: K, value: V) {
            pairs.push((s!(key), s!(value)));
        }

        let mut pairs: Vec<(String, String)> = vec![];

        if let Some((entry, index)) = self.entries.current(&self.pointer) {
            match entry {
                File(ref path) => {
                    push(&mut pairs, "file", path_to_str(path));
                }
                Http(ref path, ref url) => {
                    push(&mut pairs, "file", path_to_str(path));
                    push(&mut pairs, "url", url);
                }
                Archive(ref archive_file, ref entry) => {
                    push(&mut pairs, "file", entry.name.clone());
                    push(&mut pairs, "archive_file", path_to_str(archive_file));
                }
            }
            push(&mut pairs, "index", index + 1);
            push(&mut pairs, "count", self.entries.len());
        }

        pairs
    }

    fn get_meta(&self, entry: &Entry) -> Result<GenericMetadata, immeta::Error> {
        match *entry {
            Entry::File(ref path) | Entry::Http(ref path, _) =>
                immeta::load_from_file(&path),
            Entry::Archive(_, ref entry) =>  {
                immeta::load_from_buf(&entry.content)
            }
        }
    }

    fn puts_event_with_current(&self, event: &str, data: Option<&[(String, String)]>) {
        let mut pairs = vec![(o!("event"), o!(event))];
        pairs.extend_from_slice(self.current_info().as_slice());
        if let Some(data) = data {
            pairs.extend_from_slice(data);
        }
        output::puts(&pairs);
    }

    fn show_image1(&self, entry: Entry, image: &Image, width: i32, height: i32) {
        if let Ok(img) = self.get_meta(&entry) {
            if let Ok(gif) = img.into::<Gif>() {
                if gif.is_animated() {
                    match image_buffer::get_pixbuf_animation(&entry) {
                        Ok(buf) => image.set_from_animation(&buf),
                        Err(error) => error.show(image, width, height, &self.gui.colors.error, &self.gui.colors.error_background)
                    }
                    return
                }
            }
        }

        match self.cherenkoved.get_pixbuf(&entry, width, height) {
            Ok(buf) => {
                image.set_from_pixbuf(Some(&buf));
            },
            Err(error) => error.show(image, width, height, &self.gui.colors.error, &self.gui.colors.error_background)
        }
    }

    fn show_image(&mut self) {
        let (width, height) = self.gui.get_cell_size(&self.states.view, self.states.status_bar);

        for (index, image) in self.gui.images(self.states.reverse).enumerate() {
            if let Some(entry) = self.entries.current_with(&self.pointer, index).map(|(entry,_)| entry) {
                self.show_image1(entry, image, width, height);
            } else {
                image.set_from_pixbuf(None);
            }
        }
    }

    fn update_env(&mut self) {
        let mut new_keys = HashSet::<String>::new();
        for (name, value) in self.current_env() {
            new_keys.insert(o!(name));
            env::set_var(constant::env_name(name), value);
        }
        for name in self.current_env_keys.difference(&new_keys) {
            env::remove_var(name);
        }
        self.current_env_keys = new_keys;
    }

    fn current_env(&self) -> HashMap<&str, String> {
        use entry::Entry::*;

        let mut envs: HashMap<&str, String> = HashMap::new();

        if let Some((entry, index)) = self.entries.current(&self.pointer) {
            match entry {
                File(ref path) => {
                    envs.insert("file", o!(path_to_str(path)));
                }
                Http(ref path, ref url) => {
                    envs.insert("file", o!(path_to_str(path)));
                    envs.insert("url", o!(url));
                }
                Archive(ref archive_file, ref entry) => {
                    envs.insert("file", entry.name.clone());
                    envs.insert("archive_file", o!(path_to_str(archive_file)));
                }
            }
            envs.insert("index", s!(index + 1));
            envs.insert("count", s!(self.entries.len()));
        }

        envs
    }

    fn update_label(&self, text: &str) {
        self.gui.window.set_title(text);
        if self.states.status_bar {
            self.gui.label.set_text(text);
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
