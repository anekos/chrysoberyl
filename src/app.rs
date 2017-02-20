
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::spawn;
use std::process::exit;
use gtk::prelude::*;
use gtk::{Image, Window};
use gdk_pixbuf::{Pixbuf, PixbufAnimation};

use index_pointer::IndexPointer;
use http_cache::HttpCache;
use options::{AppOptions, AppOptionName};
use log;



pub struct App {
    index_pointer: IndexPointer,
    http_cache: HttpCache,
    files: Vec<String>,
    window: Window,
    image: Image,
    pub tx: Sender<Operation>,
    pub options: AppOptions
}


#[derive(Clone, Debug)]
pub enum Operation {
    First,
    Next,
    Previous,
    Last,
    Refresh,
    Push(String),
    PushFile(String),
    PushURL(String),
    Key(u32),
    Count(u8),
    Toggle(AppOptionName),
    Exit
}


impl App {
    pub fn new(files: Vec<String>, window: Window, image: Image) -> (App, Receiver<Operation>) {
        let (tx, rx) = channel();

        let app = App {
            index_pointer: IndexPointer::new(),
            http_cache: HttpCache::new(),
            files: vec![],
            window: window,
            image: image,
            tx: tx.clone(),
            options: AppOptions::new()
        };

        for file in files {
            tx.send(Operation::Push(file)).unwrap();
        }

        (app, rx)
    }

    pub fn operate(&mut self, operation: Operation) {
        use self::Operation::*;

        let mut next_index = None;
        let len = self.files.len();

        {
            match operation {
                First => next_index = self.index_pointer.first(len),
                Next => next_index = self.index_pointer.next(len),
                Previous => next_index = self.index_pointer.previous(),
                Last => next_index = self.index_pointer.last(len),
                Refresh => next_index = self.index_pointer.current,
                Push(path) => on_push(self.tx.clone(), path),
                PushFile(file) => on_push_file(self.tx.clone(), &mut self.files, file),
                PushURL(url) => on_push_url(self.tx.clone(), &mut self.http_cache, url),
                Key(key) => if let Some(current) = self.index_pointer.current {
                    on_key(key, self.files.get(current));
                },
                Toggle(AppOptionName::ShowText) => {
                    self.options.show_text = !self.options.show_text;
                    next_index = self.index_pointer.current;
                }
                Count(value) => self.index_pointer.push_counting_number(value),
                Exit => exit(0),
            }
        }

        if let Some(next_index) = next_index {
            if let Some(file) = self.files.get(next_index) {
                let text = &format!("[{}/{}] {}", next_index + 1, len, file);
                self.window.set_title(text);
                show_image(
                    &mut self.window,
                    &mut self.image,
                    file.clone(),
                    if self.options.show_text { Some(text) } else { None });
            }
        }
    }
}

impl AppOptions {
    fn new() -> AppOptions {
        AppOptions { show_text: false }
    }
}


fn show_image(window: &mut Window, image: &mut Image, file: String, text: Option<&str>) {
    use std::path::Path;

    log::puts1("Show", &file);

    let (width, height) = window.get_size();
    let path = Path::new(&file);

    if let Some(extension) = path.extension() {
        if extension == "gif" {
            match PixbufAnimation::new_from_file(&file) {
                Ok(buf) => image.set_from_animation(&buf),
                Err(err) => log::error(err)
            }
            return
        }
    }

    match Pixbuf::new_from_file_at_scale(&file, width, height, true) {
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

                image.set_from_surface(&surface);
            } else {
                image.set_from_pixbuf(Some(&buf));
            }
        }
        Err(err) => log::error(err)
    }
}

fn on_key(key: u32, file: Option<&String>) {
    if let Some(file) = file {
        log::puts2("Key", key, &file);
    } else {
        log::puts1("Key", key);
    }
}

fn on_push(tx: Sender<Operation>,path: String) {
    log::puts1("Push", &path);
    if path.starts_with("http://") || path.starts_with("https://") {
        tx.send(Operation::PushURL(path)).unwrap();
    } else {
        tx.send(Operation::PushFile(path)).unwrap();
    }
}

fn on_push_file(tx: Sender<Operation>,files: &mut Vec<String>, file: String) {
    log::puts1("File", &file);
    let do_show = files.is_empty();
    files.push(file);
    if do_show {
        tx.send(Operation::First).unwrap();
    }
}

fn on_push_url(tx: Sender<Operation>, http_cache: &mut HttpCache, url: String) {
    log::puts1("URL", &url);
    let mut http_cache = http_cache.clone();
    spawn(move || {
        match http_cache.get(url) {
            Ok(file) => tx.send(Operation::PushFile(file)).unwrap(),
            Err(err) => log::error(err)
        }
    });
}
