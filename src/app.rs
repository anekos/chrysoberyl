
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread::spawn;
use std::process::exit;
use gtk::prelude::*;
use gtk::{Image, Window};
use gdk_pixbuf::{Pixbuf, PixbufAnimation};
use cairo;

use index_pointer::IndexPointer;
use http_cache::HttpCache;



pub struct App {
    index_pointer: IndexPointer,
    http_cache: HttpCache,
    files: Vec<String>,
    window: Window,
    image: Image,
    pub tx: Sender<Operation>,
}

#[derive(Clone, Debug)]
pub enum Operation {
    First,
    Next,
    Previous,
    Last,
    Refresh,
    PushFile(String),
    PushURL(String),
    Key(u32),
    Count(u8),
    Exit
}


impl App {
    pub fn new(files: Vec<String>, window: Window, image: Image) -> (App, Receiver<Operation>) {
        let (tx, rx) = channel();

        let app = App {
            index_pointer: IndexPointer::new(),
            http_cache: HttpCache::new(),
            files: files,
            window: window,
            image: image,
            tx: tx,
        };

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
                PushFile(file) => on_push_file(self.tx.clone(), &mut self.files, file),
                PushURL(url) => on_push_url(self.tx.clone(), &mut self.http_cache, url),
                Key(key) => if let Some(current) = self.index_pointer.current {
                    on_key(key, self.files.get(current));
                },
                Count(value) => self.index_pointer.push_counting_number(value),
                Exit => exit(0),
            }
        }

        if let Some(next_index) = next_index {
            if let Some(file) = self.files.get(next_index) {
                self.window.set_title(&format!("[{}/{}] {}", next_index + 1, len, file));
                show_image(&mut self.window, &mut self.image, file.clone());
            }
        }
    }
}


fn show_image(window: &mut Window, image: &mut Image, file: String) {
    use std::path::Path;

    println!("Show\t{}", file);

    let (width, height) = window.get_size();
    let path = Path::new(&file);

    if let Some(extension) = path.extension() {
        if extension == "gif" {
            match PixbufAnimation::new_from_file(&file) {
                Ok(buf) => image.set_from_animation(&buf),
                Err(err) => println!("Error\t{}", err)
            }
            return
        }
    }

    match Pixbuf::new_from_file_at_scale(&file, width, height, true) {
        Ok(buf) => {
            use cairo::{Context, ImageSurface, Format};
            use gdk::prelude::ContextExt;

            let font_size = 12.0;

            let (width, height) = (buf.get_width(), buf.get_height());

            let surface = ImageSurface::create(Format::ARgb32, width, height);
            let context = Context::new(&surface);

            context.set_source_pixbuf(&buf, 0.0, 0.0);
            context.paint();

            context.set_font_size(font_size);
            context.move_to(0.0, height as f64 - font_size);
            context.set_source_rgba(0.1, 0.1, 0.1, 1.0);
            context.show_text(&file);

            image.set_from_surface(&surface);
        }
        Err(err) => println!("Error\t{}", err)
    }
}

fn on_key(key: u32, file: Option<&String>) {
    print!("Key\t{}", key);
    if let Some(file) = file {
        println!("\t{}", file);
    } else {
        println!("");
    }
}

fn on_push_file(tx: Sender<Operation>,files: &mut Vec<String>, file: String) {
    println!("Add\t{}", file);
    let do_show = files.is_empty();
    files.push(file);
    if do_show {
        tx.send(Operation::First).unwrap();
    }
}

fn on_push_url(tx: Sender<Operation>, http_cache: &mut HttpCache, url: String) {
    let mut http_cache = http_cache.clone();
    spawn(move || {
        match http_cache.get(url) {
            Ok(file) => tx.send(Operation::PushFile(file)).unwrap(),
            Err(err) => println!("Error\t{}", err)
        }
    });
}
