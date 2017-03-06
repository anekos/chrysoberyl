
use std::sync::mpsc::{channel, Sender, Receiver};
use std::path::{Path, PathBuf};
use std::fmt;
use gtk::prelude::*;
use gtk::{Image, Window};
use gdk_pixbuf::{Pixbuf, PixbufAnimation};
use immeta;
use immeta::markers::Gif;

use entry::{EntryContainer, EntryContainerOptions};
use http_cache::HttpCache;
use options::{AppOptions, AppOptionName};
use operation::Operation;
use fragile_input::new_fragile_input;
use key::KeyData;
use utils::path_to_str;
use output;
use termination;



pub struct App {
    entries: EntryContainer,
    window: Window,
    image: Image,
    http_cache: HttpCache,
    pub tx: Sender<Operation>,
    pub options: AppOptions
}


impl App {
    pub fn new(entry_options:EntryContainerOptions, http_threads: u8, expand: bool, expand_recursive: bool, shuffle: bool, files: Vec<String>, fragiles: Vec<String>, window: Window, image: Image, options: AppOptions) -> (App, Receiver<Operation>) {
        let (tx, rx) = channel();

        let mut app = App {
            entries: EntryContainer::new(entry_options),
            window: window,
            image: image,
            tx: tx.clone(),
            http_cache: HttpCache::new(http_threads, tx.clone()),
            options: options
        };

        for file in files.iter() {
           app.on_push(file.clone());
        }

        {
            let mut expand_base = None;

            if app.entries.len() == 0 {
                if let Some(file) = files.get(0) {
                    expand_base = Path::new(file).to_path_buf().parent().map(|it| it.to_path_buf());
                }
            }

            if expand {
                tx.send(Operation::Expand(expand_base)).unwrap();
            } else if expand_recursive {
                tx.send(Operation::ExpandRecursive(expand_base)).unwrap();
            }
        }

        if shuffle {
            tx.send(Operation::Shuffle(true)).unwrap();
        }

        for fragile in fragiles.clone() {
            new_fragile_input(&fragile);
        }

        (app, rx)
    }

    pub fn operate(&mut self, operation: &Operation) {
        use self::Operation::*;

        let mut changed = false;
        let mut do_refresh = false;
        let len = self.entries.len();

        debug!("Operate\t{:?}", operation);

        {
            match *operation {
                First => changed = self.entries.pointer.first(len),
                Next => changed = self.entries.pointer.next(len),
                Previous => changed = self.entries.pointer.previous(),
                Last => changed = self.entries.pointer.last(len),
                Refresh => do_refresh = true,
                Push(ref path) => self.on_push(path.clone()),
                PushFile(ref file) => {
                    changed = self.on_push_file(file.clone());
                    do_refresh = self.options.show_text;
                }
                PushURL(ref url) => self.on_push_url(url.clone()),
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
                    for entry in self.entries.to_vec() {
                        writeln!(&mut stderr(), "{}", path_to_str(&entry)).unwrap();
                    }
                }
                Exit => termination::execute(),
            }
        }

        if let Some((file, index)) = self.entries.current() {
            if changed || do_refresh {
                let len = self.entries.len();
                let path = path_to_str(&file);
                let text = &format!("[{}/{}] {}", index + 1, len, path);

                time!("show_image" => {
                    let text: Option<&str> = if self.options.show_text { Some(&text) } else { None };
                    self.show_image(file.clone(), text);
                });

                self.window.set_title(text);
                if changed {
                    puts_event!("show", "index" => index + 1, "count" => len, "file" => path);
                }
            }
        }
    }

    // pub fn operate_multi(&mut self, operations: &[Operation]) {
    //     for op in operations {
    //         self.operate(op);
    //     }
    // }

    fn show_image(&self, path: PathBuf, text: Option<&str>) {
        let (width, height) = self.window.get_size();

        if let Ok(img) = immeta::load_from_file(&path) {
            if let Ok(gif) = img.into::<Gif>() {
                if gif.is_animated() {
                    match PixbufAnimation::new_from_file(&path.to_str().unwrap()) {
                        Ok(buf) => self.image.set_from_animation(&buf),
                        Err(err) => puts_error!("at" => "show_image", "reason" => err)
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
            Err(err) => puts_error!("at" => "show_image", "reason" => err)
        }
    }

    fn on_push(&mut self, path: String) {
        if path.starts_with("http://") || path.starts_with("https://") {
            self.tx.send(Operation::PushURL(path)).unwrap();
        } else {
            self.operate(&Operation::PushFile(Path::new(&path).to_path_buf()));
        }
    }

    fn on_push_file(&mut self, file: PathBuf) -> bool {
        self.entries.push(file)
    }

    fn on_push_url(&mut self, url: String) {
        self.http_cache.fetch(url);
    }

    fn on_key(&self, key: &KeyData) {
        self.print_with_current("key", "name", key.text());
    }

    fn on_button(&self, button: &u32) {
        self.print_with_current("button", "name", button);
    }

    fn print_with_current<T: fmt::Display>(&self, base: &str, key_name: &str, first: T) {
        if let Some(file) = self.entries.current_file() {
            puts!("event" => base, key_name => first, "file" => file.to_str().unwrap());
        } else {
            puts!("event" => base, key_name => first);
        }
    }

    fn on_user(&self, data: &Vec<(String, String)>) {
        let mut args = vec![("event".to_owned(), "user".to_owned())];
        if let Some(file) = self.entries.current_file() {
            args.push(("file".to_owned(), file.to_str().unwrap().to_owned()));
        }
        args.extend_from_slice(data.as_slice());
        output::puts(&args);
    }
}
