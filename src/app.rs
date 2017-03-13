
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender, Receiver};

use encoding::types::EncodingRef;
use gdk_pixbuf::{Pixbuf, PixbufAnimation, PixbufLoader};
use gtk::prelude::*;
use gtk::{Image, Window};
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
        let mut do_refresh = false;
        let len = self.entries.len();

        debug!("Operate\t{:?}", operation);

        {
            match *operation {
                Nop => (),
                First => changed = self.entries.pointer.first(len),
                Next => changed = self.entries.pointer.next(len),
                Previous => changed = self.entries.pointer.previous(),
                Last => changed = self.entries.pointer.last(len),
                Refresh => do_refresh = true,
                Push(ref path) => self.on_push(path.clone()),
                PushPath(ref file) => {
                    changed = self.on_push_path(file.clone());
                    do_refresh = self.options.show_text;
                }
                PushHttpCache(ref file, ref url) => {
                    changed = self.on_push_http_cache(file.clone(), url.clone());
                    do_refresh = self.options.show_text;
                }
                PushURL(ref url) => self.on_push_url(url.clone()),
                PushArchiveEntry(ref archive_path, ref entry, ref buffer) => {
                    changed = self.entries.push_archive_entry(archive_path, entry, buffer.clone());
                    do_refresh = self.options.show_text;
                },
                Key(ref key) => self.on_key(key),
                Button(ref button) => self.on_button(button),
                Toggle(AppOptionName::ShowText) => {
                    self.options.show_text = !self.options.show_text;
                    do_refresh = true;
                }
                Count(value) => self.entries.pointer.push_counting_number(value),
                Expand(ref base) => {
                    let count = self.entries.pointer.counted();
                    self.entries.expand(base.clone(), count as u8, count as u8- 1);
                    do_refresh = self.options.show_text;
                }
                ExpandRecursive(ref base) => {
                    let count = self.entries.pointer.counted();
                    self.entries.expand(base.clone(), 1, count as u8);
                    changed = self.options.show_text;
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
                }
                Quit => termination::execute(),
            }
        }

        if let Some((entry, index)) = self.entries.current() {
            if changed || do_refresh {
                let len = self.entries.len();
                let path = entry.display_path();
                let text = &format!("[{}/{}] {}", index + 1, len, path);

                time!("show_image" => {
                    let text: Option<&str> = if self.options.show_text { Some(&text) } else { None };
                    self.show_image(entry.clone(), text);
                });

                self.gui.window.set_title(text);
                if changed {
                    self.puts_event_with_current("show", None);
                }
            }
        }
    }

    // pub fn operate_multi(&mut self, operations: &[Operation]) {
    //     for op in operations {
    //         self.operate(op);
    //     }
    // }

    fn show_image(&self, entry: Entry, text: Option<&str>) {
        let (width, height) = self.gui.window.get_size();

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
            Ok(buf) => {
                if let Some(text) = text {
                    use cairo::{Context, ImageSurface, Format};
                    use gdk::prelude::ContextExt;

                    let (width, height) = (buf.get_width(), buf.get_height());

                    let surface = ImageSurface::create(Format::ARgb32, width, height);

                    {
                        let height = height as f64;

                        let context = Context::new(&surface);
                        let alpha = 0.8;

                        context.set_source_pixbuf(&buf, 0.0, 0.0);
                        context.paint();

                        let font_size = 12.0;
                        context.set_font_size(font_size);

                        let text_y = {
                            let extents = context.text_extents(&text);
                            context.set_source_rgba(0.0, 0.25, 0.25, alpha);
                            context.rectangle(
                                0.0,
                                height - extents.height - 4.0,
                                extents.x_bearing + extents.x_advance + 2.0,
                                height);
                            context.fill();
                            height - 4.0
                        };

                        context.move_to(2.0, text_y);
                        context.set_source_rgba(1.0, 1.0, 1.0, alpha);
                        context.show_text(text);
                    }

                    self.gui.image.set_from_surface(&surface);
                } else {
                    self.gui.image.set_from_pixbuf(Some(&buf));
                }
            }
            Err(err) => puts_error!("at" => "show_image", "reason" => err)
        }
    }

    fn on_push(&mut self, path: String) {
        if path.starts_with("http://") || path.starts_with("https://") {
            self.tx.send(Operation::PushURL(path)).unwrap();
            return;
        }

        let path = Path::new(&path).canonicalize().expect("canonicalize");
        if let Some(ext) = path.extension() {
            match &*ext.to_str().unwrap().to_lowercase() {
                "zip" | "rar" | "tar.gz" | "lzh" | "lha" =>
                    archive::fetch_entries(&path, &self.encodings, self.tx.clone()),
                _ => ()
            }
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

    fn puts_event_with_current(&self, event: &str, data: Option<&Vec<(String, String)>>) {
        use entry::Entry::*;

        puts_with!(pairs => {
            push_pair!(pairs, "event" => event);

            if let Some((entry, index)) = self.entries.current() {
                match entry {
                    File(ref path) => push_pair!(pairs, "file" => path_to_str(path)),
                    Http(ref path, ref url) => push_pair!(pairs, "file" => path_to_str(path), "url" => url),
                    Archive(ref archive_file, ref entry, _) => push_pair!(pairs, "file" => entry.name, "archive_file" => path_to_str(archive_file)),
                }
                push_pair!(pairs, "index" => index + 1, "count" => self.entries.len());
            }

            if let Some(data) = data {
                pairs.extend_from_slice(data.as_slice());
            }
        });
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
