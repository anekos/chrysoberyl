
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::spawn;
use std::path::{Path, PathBuf};
use std::process::exit;
use gtk::prelude::*;
use gtk::{Image, Window};
use gdk_pixbuf::{Pixbuf, PixbufAnimation};

use entry::EntryContainer;
use http_cache::HttpCache;
use options::{AppOptions, AppOptionName};
use operation::Operation;
use log;
use path;



pub struct App {
    entries: EntryContainer,
    http_cache: HttpCache,
    window: Window,
    image: Image,
    pub tx: Sender<Operation>,
    pub options: AppOptions
}


impl App {
    pub fn new(files: Vec<String>, window: Window, image: Image) -> (App, Receiver<Operation>) {
        let (tx, rx) = channel();

        let app = App {
            entries: EntryContainer::new(),
            http_cache: HttpCache::new(),
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

        let mut changed = false;
        let len = self.entries.len();

        {
            match operation {
                First => changed = self.entries.pointer.first(len),
                Next => changed = self.entries.pointer.next(len),
                Previous => changed = self.entries.pointer.previous(),
                Last => changed = self.entries.pointer.last(len),
                Refresh => changed = true,
                Push(ref path) => on_push(self.tx.clone(), path.clone()),
                PushFile(ref file) => {
                    on_push_file(self.tx.clone(), &mut self.entries, file.clone());
                    changed = self.options.show_text;
                },
                PushURL(ref url) => on_push_url(self.tx.clone(), &mut self.http_cache, url.clone()),
                Key(_) => (),
                Toggle(AppOptionName::ShowText) => {
                    self.options.show_text = !self.options.show_text;
                    changed = true;
                }
                Count(value) => self.entries.pointer.push_counting_number(value),
                Expand => self.entries.expand(),
                Exit => exit(0),
            }
        }

        operation.log(self.entries.current_file());

        if changed {
            if let Some((file, index)) = self.entries.current() {
                let len = self.entries.len();
                let text = &format!("[{}/{}] {}", index + 1, len, path::to_string(&file));
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


fn show_image(window: &mut Window, image: &mut Image, path: PathBuf, text: Option<&str>) {
    let (width, height) = window.get_size();

    if let Some(extension) = path.extension() {
        if extension == "gif" {
            match PixbufAnimation::new_from_file(&path.to_str().unwrap()) {
                Ok(buf) => image.set_from_animation(&buf),
                Err(err) => log::error(err)
            }
            return
        }
    }

    match Pixbuf::new_from_file_at_scale(&path.to_str().unwrap(), width, height, true) {
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

fn on_push(tx: Sender<Operation>, path: String) {
    if path.starts_with("http://") || path.starts_with("https://") {
        tx.send(Operation::PushURL(path)).unwrap();
    } else {
        tx.send(Operation::PushFile(Path::new(&path).to_path_buf())).unwrap();
    }
}

fn on_push_file(tx: Sender<Operation>, entries: &mut EntryContainer, file: PathBuf) {
    let do_show = entries.is_empty();
    entries.push(file);
    if do_show {
        tx.send(Operation::First).unwrap();
    }
}

fn on_push_url(tx: Sender<Operation>, http_cache: &mut HttpCache, url: String) {
    let mut http_cache = http_cache.clone();
    spawn(move || {
        match http_cache.get(url) {
            Ok(file) => tx.send(Operation::PushFile(file)).unwrap(),
            Err(err) => log::error(err)
        }
    });
}
