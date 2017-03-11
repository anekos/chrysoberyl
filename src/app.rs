
use std::sync::mpsc::{channel, Sender, Receiver};
use std::path::{Path, PathBuf};
use gtk::prelude::*;
use gtk::{Image, Window};
use immeta::markers::Gif;

use entry::{Entry,EntryContainer, EntryContainerOptions};
use http_cache::HttpCache;
use options::{AppOptions, AppOptionName};
use operation::Operation;
use fragile_input::new_fragile_input;
use key::KeyData;
use utils::path_to_str;
use output;
use termination;
use mapping::{Mapping, Input};
use pixbuf::*;



pub struct App {
    entries: EntryContainer,
    window: Window,
    image: Image,
    http_cache: HttpCache,
    mapping: Mapping,
    pub tx: Sender<Operation>,
    pub options: AppOptions
}


impl App {
    pub fn new(entry_options:EntryContainerOptions, http_threads: u8, expand: bool, expand_recursive: bool, shuffle: bool, files: Vec<String>, fragiles: Vec<String>, window: Window, image: Image, options: AppOptions) -> (App, Receiver<Operation>) {
        let (tx, rx) = channel();

        let mut entry_options = entry_options;

        if entry_options.encodings.is_empty() {
            use encoding::all::*;
            entry_options.encodings.push(UTF_8);
            entry_options.encodings.push(WINDOWS_31J);
        }

        let mut app = App {
            entries: EntryContainer::new(entry_options),
            window: window,
            image: image,
            tx: tx.clone(),
            http_cache: HttpCache::new(http_threads, tx.clone()),
            options: options,
            mapping: Mapping::new()
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
            let fix = files.get(0).map(|it| Path::new(it).is_file()).unwrap_or(false);
            tx.send(Operation::Shuffle(fix)).unwrap();
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

                self.window.set_title(text);
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
        let (width, height) = self.window.get_size();

        if let Ok(img) = get_meta(&entry) {
            if let Ok(gif) = img.into::<Gif>() {
                if gif.is_animated() {
                    match get_pixbuf_animation(&entry) {
                        Ok(buf) => self.image.set_from_animation(&buf),
                        Err(err) => puts_error!("at" => "show_image", "reason" => err)
                    }
                    return
                }
            }
        }

        match get_pixbuf(&&entry, width, height) {
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
            self.operate(&Operation::PushPath(Path::new(&path).to_path_buf()));
        }
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

    fn on_key(&self, key: &KeyData) {
        let key_name = key.text();
        if let Some(op) = self.mapping.matched(&Input::key(&key_name)) {
            self.tx.send(op).unwrap();
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
                    Archive(ref archive_file, ref file, _) => push_pair!(pairs, "file" => file, "archive_file" => path_to_str(archive_file)),
                }
                push_pair!(pairs, "index" => index + 1, "count" => self.entries.len());
            }

            if let Some(data) = data {
                pairs.extend_from_slice(data.as_slice());
            }
        });
    }
}
