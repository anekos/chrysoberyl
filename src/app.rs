
use std::fs::remove_file;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::path::{Path, PathBuf};
use std::process::exit;
use ctrlc;
use gtk::prelude::*;
use gtk::{Image, Window};
use gdk_pixbuf::{Pixbuf, PixbufAnimation};
use immeta;
use immeta::markers::Gif;

use entry::{EntryContainer, EntryContainerOptions};
use http_cache::HttpCache;
use options::{AppOptions, AppOptionName};
use operation::Operation;
use output;
use path;
use fragile_input::new_fragile_input;
use key::KeyData;



pub struct App {
    entries: EntryContainer,
    window: Window,
    image: Image,
    fragiles: Vec<String>,
    previous_len: usize,
    http_cache: HttpCache,
    pub tx: Sender<Operation>,
    pub options: AppOptions
}


impl App {
    pub fn new(entry_options:EntryContainerOptions, files: Vec<String>, fragiles: Vec<String>, window: Window, image: Image) -> (App, Receiver<Operation>) {
        let (tx, rx) = channel();
        let options = AppOptions::new();

        let mut app = App {
            entries: EntryContainer::new(entry_options),
            window: window,
            image: image,
            tx: tx.clone(),
            http_cache: HttpCache::new(tx.clone()),
            previous_len: 0,
            fragiles: fragiles.clone(),
            options: options
        };

        for file in files {
           app.on_push(file);
        }

        for fragile in fragiles.clone() {
            new_fragile_input(&fragile);
        }

        ctrlc::set_handler(move || on_exit(fragiles.clone()));

        (app, rx)
    }

    pub fn operate(&mut self, operation: &Operation) {
        use self::Operation::*;

        let mut changed = false;
        let len = self.entries.len();

        debug!("Operate\t{:?}", operation);

        {
            match *operation {
                First => changed = self.entries.pointer.first(len),
                Next => changed = self.entries.pointer.next(len),
                Previous => changed = self.entries.pointer.previous(),
                Last => changed = self.entries.pointer.last(len),
                Refresh => changed = true,
                Push(ref path) => self.on_push(path.clone()),
                PushFile(ref file) => {
                    self.on_push_file(file.clone());
                    changed = self.options.show_text;
                }
                PushURL(ref url) => self.on_push_url(url.clone()),
                Key(ref key) => self.on_key(key),
                Toggle(AppOptionName::ShowText) => {
                    self.options.show_text = !self.options.show_text;
                    changed = true;
                }
                Count(value) => self.entries.pointer.push_counting_number(value),
                Expand => {
                    let count = self.entries.pointer.counted();
                    self.entries.expand(count as u8, count as u8- 1);
                    changed = self.options.show_text;
                }
                ExpandRecursive => {
                    let count = self.entries.pointer.counted();
                    self.entries.expand(1, count as u8);
                    changed = self.options.show_text;
                }
                Shuffle => {
                    self.entries.shuffle();
                    changed = true;
                }
                Sort => {
                    self.entries.sort();
                    changed = true;
                }
                Exit => self.on_exit(),
            }
        }

        if let Some((file, index)) = self.entries.current() {
            if changed {
                let len = self.entries.len();
                let text = &format!("[{}/{}] {}", index + 1, len, path::to_string(&file));
                self.window.set_title(text);
                self.show_image(
                    file.clone(),
                    if self.options.show_text { Some(text) } else { None });
            }
        } else {
            let len = self.entries.len();
            if self.previous_len == 0 && len > 0 {
                self.entries.pointer.first(len);
            }
            self.previous_len = len;
        }
    }

    pub fn operate_multi(&mut self, operations: &[Operation]) {
        for op in operations {
            self.operate(op);
        }
    }

    fn show_image(&self, path: PathBuf, text: Option<&str>) {
        let (width, height) = self.window.get_size();

        if let Ok(img) = immeta::load_from_file(&path) {
            if let Ok(gif) = img.into::<Gif>() {
                if gif.is_animated() {
                    match PixbufAnimation::new_from_file(&path.to_str().unwrap()) {
                        Ok(buf) => self.image.set_from_animation(&buf),
                        Err(err) => output::error(err)
                    }
                    return
                }
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

                    self.image.set_from_surface(&surface);
                } else {
                    self.image.set_from_pixbuf(Some(&buf));
                }
            }
            Err(err) => output::error(err)
        }
    }

    fn on_push(&mut self, path: String) {
        if path.starts_with("http://") || path.starts_with("https://") {
            self.tx.send(Operation::PushURL(path)).unwrap();
        } else {
            self.operate(&Operation::PushFile(Path::new(&path).to_path_buf()));
        }
    }

    fn on_push_file(&mut self, file: PathBuf) {
        self.entries.push(file);
    }

    fn on_push_url(&mut self, url: String) {
        self.http_cache.fetch(url);
    }

    fn on_exit(&self) {
        on_exit(self.fragiles.clone());
    }

    fn on_key(&self, key: &KeyData) {
        print!("Key\t{}", key.text());
        if let Some(file) = self.entries.current_file() {
            println!("\t{}", file.to_str().unwrap());
        } else {
            println!("");
        }
    }
}


fn on_exit(fragiles: Vec<String>) {
    for fragile in fragiles {
        remove_file(fragile).unwrap();
    }
    exit(0);
}
