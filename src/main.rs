
extern crate gtk;
extern crate gdk;
extern crate gdk_pixbuf;


use gdk_pixbuf::{Pixbuf, PixbufAnimation};
use gtk::prelude::*;
use gtk::{Image, Window};
use std::env::args;
use std::sync::mpsc::{channel, Sender};
use std::thread::{sleep, spawn};
use std::time::Duration;


#[derive(Clone, Debug)]
enum Operation {
    First,
    Next,
    Previous,
    Last,
    Refresh,
    Append(String),
    Key(u32),
    Count(i64),
    Exit
}


fn main() {
    use self::Operation::*;

    let (window, mut image) = setup();

    let mut files: Vec<String> = args().skip(1).collect();

    let (tx, rx) = channel();

    {
        let tx = tx.clone();
        window.connect_key_press_event(move |_, key| on_key_press(tx.clone(), key));
    }

    {
        let tx = tx.clone();
        window.connect_configure_event(move |_, _| on_configure(tx.clone()));
    }

    {
        let tx = tx.clone();
        stdin_reader(tx);
    }

    window.show_all();

    tx.send(First).unwrap();

    {

        let mut index: i64 = 0;
        let mut count: Option<i64> = None;

        loop {
            while gtk::events_pending() {
                gtk::main_iteration();
            }

            for operation in rx.try_iter() {
                let mut next_index = None;

                match operation {
                    First => { next_index = Some(counted(&mut count) - 1); },
                    Next => { next_index = Some(index + counted(&mut count)); },
                    Previous => { next_index = Some(index - counted(&mut count)); },
                    Last => { next_index = Some(files.len() as i64 - counted(&mut count) + 1); },
                    Refresh => { next_index = Some(index); },
                    Append(file) => {
                        println!("Add\t{}", file);
                        let do_show = files.is_empty();
                        files.push(file);
                        if do_show {
                            tx.send(First).unwrap();
                        }
                    }
                    Key(key) => {
                        print!("Key\t{}", key);
                        if let Some(file) = files.get(index as usize) {
                            println!("\t{}", file);
                        } else {
                            println!("");
                        }
                    }
                    Count(value) => {
                        if let Some(current) = count {
                            count = Some(current * 10 + value);
                        } else {
                            count = Some(value);
                        }
                    }
                    Exit => { std::process::exit(0); }
                }

                if let Some(next_index) = next_index {
                    if 0 <= next_index && next_index < files.len() as i64 {
                        index = next_index;
                        let file = files[index as usize].clone();
                        window.set_title(&format!("[{}/{}] {}", index + 1, files.len(), file));
                        show_image(&window, &mut image, file);
                    }
                }
            }

            sleep(Duration::from_millis(10));
        }
    }
}


fn show_image(window: &Window, image: &mut Image, file: String) {
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
        Ok(buf) => image.set_from_pixbuf(Some(&buf)),
        Err(err) => println!("Error\t{}", err)
    }
}


fn on_configure(tx: Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}


fn on_key_press(tx: Sender<Operation>, key: &gdk::EventKey) -> gtk::Inhibit {
    use self::Operation::*;

    if let Some(operation) = match key.as_ref().keyval {
        104 | 102 => Some(First),
        106 => Some(Next),
        107 => Some(Previous),
        108 => Some(Last),
        113 => Some(Exit),
        114 => Some(Refresh),
        key => if 48 <= key && key <= 57 {
            Some(Count((key - 48) as i64))
        } else {
            Some(Key(key))
        }
    } {
        tx.send(operation).unwrap();
    }

    Inhibit(false)
}


fn stdin_reader(tx: Sender<Operation>) {
    use std::io;
    use std::io::BufRead;

    spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
             tx.send(Operation::Append(line.unwrap())).unwrap();
             // tx.send(Operation::Last).unwrap();
        }
    });
}


fn setup() -> (Window, Image) {
    gtk::init().unwrap();

    let window = gtk::Window::new(gtk::WindowType::Toplevel);

    window.set_title("Chrysoberyl");
    window.set_border_width(0);
    window.set_position(gtk::WindowPosition::Center);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let image = Image::new_from_pixbuf(None);
    window.add(&image);

    (window, image)
}


fn counted(count: &mut Option<i64>) -> i64 {
    let result = count.unwrap_or(1);
    *count = None;
    result
}
