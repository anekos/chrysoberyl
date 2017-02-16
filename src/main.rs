
extern crate gtk;
extern crate gdk;
extern crate gdk_pixbuf;


use std::env::args;
use std::sync::mpsc::{channel, Sender};
use gtk::prelude::*;
use gtk::{Image, Window};
use gdk_pixbuf::{Pixbuf, PixbufAnimation};


#[derive(Clone, Debug)]
enum Operation {
    First,
    Next,
    Previous,
    Refresh,
    Exit
}


fn main() {
    use self::Operation::*;

    let (window, mut image) = setup();

    let files: Vec<String> = args().skip(1).collect();

    let (tx, rx) = channel();

    {
        let tx = tx.clone();
        window.connect_key_press_event(move |_, key| on_key_press(tx.clone(), key));
    }

    {
        let tx = tx.clone();
        window.connect_configure_event(move |_, _| on_configure(tx.clone()));
    }

    window.show_all();

    tx.send(First).unwrap();

    {

        let mut index: i64 = 0;

        loop {
            while gtk::events_pending() {
                gtk::main_iteration();
            }

            for operation in rx.try_iter() {
                let next_index;
                match operation {
                    First => { next_index = Some(0); },
                    Next => { next_index = Some(index + 1); },
                    Previous => { next_index = Some(index - 1); },
                    Refresh => { next_index = Some(index); },
                    Exit => { std::process::exit(0); }
                }

                if let Some(next_index) = next_index {
                    if 0 <= next_index && next_index < files.len() as i64 {
                        index = next_index;
                        show_image(&window, &mut image, files[index as usize].clone());
                    }
                }
            }
        }
    }
}


fn show_image(window: &Window, image: &mut Image, file: String) {
    use std::path::Path;

    let (width, height) = window.get_size();
    let path = Path::new(&file);

    if let Some(extension) = path.extension() {
        if extension == "gif" {
            match PixbufAnimation::new_from_file(&file) {
                Ok(buf) => image.set_from_animation(&buf),
                Err(err) => println!("Error: {}", err)
            }
            return
        }
    }

    match Pixbuf::new_from_file_at_scale(&file, width, height, true) {
        Ok(buf) => image.set_from_pixbuf(Some(&buf)),
        Err(err) => println!("Error: {}", err)
    }
}


fn on_configure(tx: Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}


fn on_key_press(tx: Sender<Operation>, key: &gdk::EventKey) -> gtk::Inhibit {
    use self::Operation::*;

    if let Some(operation) = match key.as_ref().keyval {
        104 => Some(First),
            106 => Some(Next),
            107 => Some(Previous),
            113 => Some(Exit),
            114 => Some(Refresh),
            _ => None
    } {
        tx.send(operation).unwrap();
    }

    Inhibit(false)
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
