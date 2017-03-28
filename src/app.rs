
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc::{channel, Sender, Receiver};

use encoding::types::EncodingRef;
use gtk::prelude::*;
use gtk::Image;
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};
use rand::{self, ThreadRng};
use rand::distributions::{IndependentSample, Range};

use archive::{self, ArchiveEntry};
use color::RGB;
use command;
use constant;
use controller;
use entry::{Entry,EntryContainer, EntryContainerOptions};
use events;
use fragile_input::new_fragile_input;
use gui::{Gui, ColorTarget};
use http_cache::HttpCache;
use image_buffer;
use index_pointer::IndexPointer;
use mapping::{Mapping, Input};
use operation::{Operation, OptionModifier};
use options::{AppOptions, AppOptionName};
use output;
use shell;
use termination;
use utils::path_to_str;



pub struct App {
    entries: EntryContainer,
    mapping: Mapping,
    http_cache: HttpCache,
    encodings: Vec<EncodingRef>,
    gui: Gui,
    draw_serial: u64,
    rng: ThreadRng,
    pointer: IndexPointer,
    pub tx: Sender<Operation>,
    pub options: AppOptions
}

pub struct Initial {
    pub http_threads: u8,
    pub expand: bool,
    pub expand_recursive: bool,
    pub shuffle: bool,
    pub controllers: controller::Controllers,
    pub files: Vec<String>,
    pub encodings: Vec<EncodingRef>,
    pub before: Vec<String>
}

struct Updated {
    pointer: bool,
    label: bool,
    image: bool
}


impl App {
    pub fn new(initial: Initial, options: AppOptions, gui: Gui, entry_options:EntryContainerOptions) -> (App, Receiver<Operation>, Receiver<Operation>) {
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
            gui: gui.clone(),
            tx: tx.clone(),
            http_cache: HttpCache::new(initial.http_threads, tx.clone()),
            options: options,
            encodings: initial.encodings,
            mapping: Mapping::new(),
            draw_serial: 0,
            rng: rand::thread_rng(),
            pointer: IndexPointer::new(),
        };

        for op in &initial.before {
            match Operation::from_str(op) {
                Ok(op) => tx.send(op).unwrap(),
                Err(err) => puts_error!("at" => "before", "reason" => err),
            }
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

        for fragile in initial.controllers.fragiles.clone() {
            new_fragile_input(&fragile);
        }

        (app, primary_rx, rx)
    }

    pub fn operate(&mut self, operation: &Operation) {
        use self::Operation::*;

        let mut updated = Updated { pointer: false, label: false, image: false };
        let len = self.entries.len();

        debug!("Operate\t{:?}", operation);

        {
            match *operation {
                Clear =>
                    self.on_clear(&mut updated),
                Color(ref target, ref color) =>
                    self.on_color(&mut updated, target, color),
                Command(ref command) =>
                    self.on_command(command),
                Count(count) =>
                    self.pointer.set_count(count),
                CountDigit(digit) =>
                    self.pointer.push_count_digit(digit),
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
                Map(ref input, ref mapped_operation) =>
                    self.on_map(input, mapped_operation),
                Multi(ref ops) =>
                    self.on_multi(ops),
                Next(count) =>
                    self.on_next(&mut updated, count, len),
                Nop =>
                    (),
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
                Shell(async, read_operations, ref command_name, ref arguments) =>
                    shell::call(async, command_name, arguments, self.current_info(), option!(read_operations, self.tx.clone())),
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
            let text = self.options.show_text;
            time!("show_image" => self.show_image(text));
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
        }
    }

    /* Operation event */

    fn on_clear(&mut self, updated: &mut Updated) {
        self.entries.clear(&mut self.pointer);
        updated.image = true;
    }

    fn on_color(&mut self, updated: &mut Updated, target: &ColorTarget, color: &RGB) {
        use self::ColorTarget::*;

        self.gui.update_color(target, color);

        updated.image = match *target {
            Error | ErrorBackground => true,
            _ => false
        };
    }

    fn on_command(&mut self, command: &command::Command) {
        use entry::Entry::*;

        if let Some((entry, _)) = self.entries.current(&self.pointer) {
            let result = match entry {
                File(ref path) | Http(ref path, _) => command.execute(path),
                Archive(_ , _) => Err(o!("copy/move does not support archive files."))
            };
            let command_str = format!("{:?}", command);
            match result {
                Ok(_) => puts_event!("command", "status" => "ok", "command" => command_str),
                Err(err) => puts_event!("command", "status" => "fail", "reason" => err, "command" => command_str),
            }
        }
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

        if let Some(op) = self.mapping.matched(input) {
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

    fn on_map(&mut self, input: &Input, operation: &Box<Operation>) {
        // FIXME
        puts_event!("map",
                    "input" => format!("{:?}", input),
                    "operation" => format!("{:?}", operation));
        self.mapping.register(input.clone(), *operation.clone());
    }

    fn on_multi(&mut self, operations: &[Operation]) {
        for op in operations {
            self.operate(op)
        }
    }

    fn on_next(&mut self, updated: &mut Updated, count: Option<usize>, len: usize) {
        updated.pointer = self.pointer.with_count(count).next(len);
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

    fn on_update_option(&mut self, updated: &mut Updated, name: &AppOptionName, modifier: &OptionModifier) {
        use options::AppOptionName::*;
        use self::OptionModifier::*;

        {
            let value: &mut bool = match *name {
                ShowText => &mut self.options.show_text,
                Reverse => &mut self.options.reverse,
                CenterAlignment => &mut self.options.center_alignment,
            };

            match *modifier {
                Toggle => *value ^= true,
                Enable => *value = true,
                Disable => *value = false,
            }
        }

        if let ShowText = *name {
            self.update_label_visibility()
        }

        updated.image = true;
    }

    fn on_user(&self, data: &[(String, String)]) {
        self.puts_event_with_current("user", Some(data));
    }

    fn on_views(&mut self, updated: &mut Updated, cols: Option<usize>, rows: Option<usize>) {
        updated.image = self.gui.reset_images(cols, rows);
        self.pointer.multiply(self.gui.len());
    }

    fn on_views_fellow(&mut self, updated: &mut Updated, for_rows: bool) {
        let size = self.pointer.counted();
        updated.image = if for_rows {
            self.gui.reset_images(None, Some(size))
        } else {
            self.gui.reset_images(Some(size), None)
        };
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

        match image_buffer::get_pixbuf(&entry, width, height) {
            Ok(buf) => {
                image.set_from_pixbuf(Some(&buf));
            },
            Err(error) => error.show(image, width, height, &self.gui.colors.error, &self.gui.colors.error_background)
        }
    }

    fn show_image(&mut self, with_label: bool) {
        let (width, height) = self.gui.get_cell_size(with_label);
        let images_len = self.gui.len();

        for (mut index, image) in self.gui.images().enumerate() {
            if self.options.reverse {
                index = images_len - index - 1;
            }
            if let Some(entry) = self.entries.current_with(&self.pointer, index).map(|(entry,_)| entry) {
                self.show_image1(entry, image, width, height);
            } else {
                image.set_from_pixbuf(None);
            }
        }

        self.gui.set_center_alignment(self.options.center_alignment);
    }

    fn update_label(&self, text: &str) {
        self.gui.window.set_title(text);
        if self.options.show_text {
            self.gui.label.set_text(text);
        }
    }

    fn update_label_visibility(&self) {
        if self.options.show_text {
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
            before: vec![],
        }
    }
}
