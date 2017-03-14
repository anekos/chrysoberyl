
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};

use encoding::types::EncodingRef;
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};
use gtk::prelude::*;
use gtk::{Image, Window, Label};
use gtk;
use immeta::markers::Gif;
use immeta::{self, GenericMetadata};

use archive;
use controller;
use entry::{Entry,EntryContainer, EntryContainerOptions};
use events;
use fragile_input::new_fragile_input;
use http_cache::HttpCache;
use key::KeyData;
use mapping::{Mapping, Input};
use operation::Operation;
use options::{AppOptions, AppOptionName};
use output;
use script;
use termination;
use utils::path_to_str;



pub struct App {
    entries: EntryContainer,
    mapping: Mapping,
    http_cache: HttpCache,
    encodings: Vec<EncodingRef>,
    gui: Gui,
    pub tx: Sender<Operation>,
    pub options: AppOptions
}

#[derive(Clone)]
pub struct Gui {
    pub window: Window,
    pub image: Image,
    pub label: Label,
}

pub struct Initial {
    pub http_threads: u8,
    pub expand: bool,
    pub expand_recursive: bool,
    pub shuffle: bool,
    pub controllers: controller::Controllers,
    pub files: Vec<String>,
    pub encodings: Vec<EncodingRef>
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
            mapping: Mapping::new()
        };

        events::register(gui, primary_tx.clone());
        controller::register(tx.clone(), &initial.controllers);

        app.update_label_visibility();

        for file in initial.files.iter() {
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
                tx.send(Operation::Expand(expand_base)).unwrap();
            } else if initial.expand_recursive {
                tx.send(Operation::ExpandRecursive(expand_base)).unwrap();
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

        let mut changed = false;
        let mut label_updated = false;
        let len = self.entries.len();

        debug!("Operate\t{:?}", operation);

        {
            match *operation {
                Nop => (),
                First => changed = self.entries.pointer.first(len),
                Next => changed = self.entries.pointer.next(len),
                Previous => changed = self.entries.pointer.previous(),
                Last => changed = self.entries.pointer.last(len),
                Refresh => changed = true,
                Push(ref path) => self.on_push(path.clone()),
                PushPath(ref file) => {
                    changed = self.on_push_path(file.clone());
                    label_updated = true;
                }
                PushHttpCache(ref file, ref url) => {
                    changed = self.on_push_http_cache(file.clone(), url.clone());
                    label_updated = true;
                }
                PushURL(ref url) => self.on_push_url(url.clone()),
                PushArchiveEntry(ref archive_path, ref entry, ref buffer) => {
                    changed = self.entries.push_archive_entry(archive_path, entry, buffer.clone());
                    label_updated = true;
                },
                Key(ref key) => self.on_key(key),
                Button(ref button) => self.on_button(button),
                Toggle(AppOptionName::ShowText) => {
                    self.options.show_text = !self.options.show_text;
                    self.update_label_visibility();
                    changed = true;
                }
                Count(count) => self.entries.pointer.set_count(count),
                CountDigit(digit) => self.entries.pointer.push_count_digit(digit),
                Expand(ref base) => {
                    let count = self.entries.pointer.counted();
                    self.entries.expand(base.clone(), count as u8, count as u8- 1);
                    label_updated = true;
                }
                ExpandRecursive(ref base) => {
                    let count = self.entries.pointer.counted();
                    self.entries.expand(base.clone(), 1, count as u8);
                    label_updated = true;
                }
                Shuffle(fix_current) => {
                    self.entries.shuffle(fix_current);
                    changed = true;
                }
                Sort => {
                    self.entries.sort();
                    changed = true;
                }
                User(ref data) => self.on_user(data),
                PrintEntries => {
                    use std::io::{Write, stderr};
                    for entry in self.entries.to_displays() {
                        writeln!(&mut stderr(), "{}", entry).unwrap();
                    }
                }
                Map(ref input, ref mapped_operation) => {
                    // FIXME
                    puts_event!("map",
                                "input" => format!("{:?}", input),
                                "operation" => format!("{:?}", mapped_operation));
                    self.mapping.register(input.clone(), *mapped_operation.clone());
                },
                Multi(ref ops) => {
                    for op in ops {
                        self.operate(op)
                    }
                },
                Script(async, ref command_name, ref arguments) => script::call(async, command_name, arguments, self.current_info()),
                Quit => termination::execute(),
            }
        }

        if let Some((entry, index)) = self.entries.current() {
            if changed {
                time!("show_image" => self.show_image(entry.clone(), self.options.show_text));
                self.puts_event_with_current("show", None);
            }

            if changed || label_updated {
                let len = self.entries.len();
                let path = entry.display_path();
                self.update_label(&format!("[{}/{}] {}", index + 1, len, path));
            }
        }
    }

    // pub fn operate_multi(&mut self, operations: &[Operation]) {
    //     for op in operations {
    //         self.operate(op);
    //     }
    // }

    fn update_label(&self, text: &str) {
        self.gui.window.set_title(text);
        if self.options.show_text {
            self.gui.label.set_text(text);
        }
    }

    fn show_image(&self, entry: Entry, with_label: bool) {
        let (width, mut height) = self.gui.window.get_size();

        if with_label {
            height -=  self.gui.label.get_allocated_height();;
        }

        if let Ok(img) = self.get_meta(&entry) {
            if let Ok(gif) = img.into::<Gif>() {
                if gif.is_animated() {
                    match self.get_pixbuf_animation(&entry) {
                        Ok(buf) => self.gui.image.set_from_animation(&buf),
                        Err(err) => puts_error!("at" => "show_image", "reason" => err)
                    }
                    return
                }
            }
        }

        match self.get_pixbuf(&&entry, width, height) {
            Ok(buf) => self.gui.image.set_from_pixbuf(Some(&buf)),
            Err(err) => puts_error!("at" => "show_image", "reason" => err)
        }
    }

    fn update_label_visibility(&self) {
        if self.options.show_text {
            self.gui.label.show();
        } else {
            self.gui.label.hide();
        }
    }

    fn on_push(&mut self, path: String) {
        if path.starts_with("http://") || path.starts_with("https://") {
            self.tx.send(Operation::PushURL(path)).unwrap();
            return;
        }

        match Path::new(&path).canonicalize() {
            Ok(path) => {
                if let Some(ext) = path.extension() {
                    match &*ext.to_str().unwrap().to_lowercase() {
                        "zip" | "rar" | "tar.gz" | "lzh" | "lha" =>
                            return archive::fetch_entries(&path, &self.encodings, self.tx.clone()),
                        _ => ()
                    }
                }

            }
            _ => ()
        }

        self.operate(&Operation::PushPath(Path::new(&path).to_path_buf()));
    }

    fn on_push_path(&mut self, file: PathBuf) -> bool {
        self.entries.push_path(&file)
    }

    fn on_push_http_cache(&mut self, file: PathBuf, url: String) -> bool {
        self.entries.push_http_cache(&file, &url)
    }

    fn on_push_url(&mut self, url: String) {
        self.http_cache.fetch(url);
    }

    fn on_key(&mut self, key: &KeyData) {
        let key_name = key.text();
        if let Some(op) = self.mapping.matched(&Input::key(&key_name)) {
            self.operate(&op);
        } else {
            self.puts_event_with_current(
                "keyboard",
                Some(&vec![("name".to_owned(), key.text().to_owned())]));
        }
    }

    fn on_button(&self, button: &u32) {
        if let Some(op) = self.mapping.matched(&Input::mouse_button(*button)) {
            self.tx.send(op).unwrap();
        } else {
            self.puts_event_with_current(
                "mouse_button",
                Some(&vec![("name".to_owned(), format!("{}", button))]));
        }
    }

    fn on_user(&self, data: &Vec<(String, String)>) {
        self.puts_event_with_current("user", Some(data));
    }

    fn current_info(&self) -> Vec<(String, String)> {
        use entry::Entry::*;
        use std::fmt::Display;

        fn push<K: Display, V: Display>(pairs: &mut Vec<(String, String)>, key: K, value: V) {
            pairs.push((s!(key), s!(value)));
        }

        let mut pairs: Vec<(String, String)> = vec![];

        if let Some((entry, index)) = self.entries.current() {
            match entry {
                File(ref path) => {
                    push(&mut pairs, "file", path_to_str(path));
                }
                Http(ref path, ref url) => {
                    push(&mut pairs, "file", path_to_str(path));
                    push(&mut pairs, "url", url);
                }
                Archive(ref archive_file, ref entry, _) => {
                    push(&mut pairs, "file", entry.name.clone());
                    push(&mut pairs, "archive_file", path_to_str(archive_file));
                }
            }
            push(&mut pairs, "index", index + 1);
            push(&mut pairs, "count", self.entries.len());
        }

        pairs
    }

    fn puts_event_with_current(&self, event: &str, data: Option<&Vec<(String, String)>>) {
        let mut pairs = vec![(s!("event"), s!(event))];
        pairs.extend_from_slice(self.current_info().as_slice());
        if let Some(data) = data {
            pairs.extend_from_slice(data.as_slice());
        }
        output::puts(&pairs);
    }

    pub fn get_pixbuf_animation(&self, entry: &Entry) -> Result<PixbufAnimation, gtk::Error> {
        match *entry {
            Entry::File(ref path) => PixbufAnimation::new_from_file(path_to_str(path)),
            Entry::Http(ref path, _) => PixbufAnimation::new_from_file(path_to_str(path)),
            Entry::Archive(_, _, ref buffer) => {
                let loader = PixbufLoader::new();
                loader.loader_write(&*buffer.as_slice()).map(|_| {
                    loader.close().unwrap();
                    loader.get_animation().unwrap()
                })
            }
        }
    }

    pub fn get_pixbuf(&self, entry: &Entry, width: i32, height: i32) -> Result<Pixbuf, gtk::Error> {
        use gdk_pixbuf::InterpType;

        match *entry {
            Entry::File(ref path) => Pixbuf::new_from_file_at_scale(path_to_str(path), width, height, true),
            Entry::Http(ref path, _) => Pixbuf::new_from_file_at_scale(path_to_str(path), width, height, true),
            Entry::Archive(_, _, ref buffer) => {
                let loader = PixbufLoader::new();
                let pixbuf = loader.loader_write(&*buffer.as_slice()).map(|_| {
                    loader.close().unwrap();
                    let source = loader.get_pixbuf().unwrap();
                    let (scale, out_width, out_height) = calculate_scale(&source, width, height);
                    let mut scaled = unsafe { Pixbuf::new(0, false, 8, out_width, out_height).unwrap() };
                    source.scale(&mut scaled, 0, 0, out_width, out_height, 0.0, 0.0, scale, scale, InterpType::Bilinear);
                    scaled
                });
                pixbuf
            }
        }
    }

    pub fn get_meta(&self, entry: &Entry) -> Result<GenericMetadata, immeta::Error> {
        match *entry {
            Entry::File(ref path) => immeta::load_from_file(&path),
            Entry::Http(ref path, _) => immeta::load_from_file(&path),
            Entry::Archive(_, _, ref buffer) =>  {
                immeta::load_from_buf(&buffer)
            }
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
        }
    }
}


fn calculate_scale(pixbuf: &Pixbuf, max_width: i32, max_height: i32) -> (f64, i32, i32) {
    let (in_width, in_height) = (pixbuf.get_width(), pixbuf.get_height());
    let mut scale = max_width as f64 / in_width as f64;
    let mut out_height = (in_height as f64 * scale) as i32;
    if out_height > max_height {
        scale = max_height as f64 / in_height as f64;
        out_height = (in_height as f64 * scale) as i32;
    }
    (scale, (in_width as f64 * scale) as i32, out_height)
}
