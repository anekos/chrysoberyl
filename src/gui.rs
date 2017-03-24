
use gtk::prelude::*;
use gtk::{self, Window, Image, Label};

use constant;



#[derive(Clone)]
pub struct Gui {
    cols: usize,
    rows: usize,
    image_outer: gtk::Box,
    image_inners: Vec<gtk::Box>,
    pub window: Window,
    pub images: Vec<Image>,
    pub label: Label,
}



impl Gui {
    pub fn new() -> Gui {
        use gtk::Orientation;

        gtk::init().unwrap();

        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        window.set_title(constant::DEFAULT_TITLE);
        window.set_border_width(0);
        window.set_position(gtk::WindowPosition::Center);

        let vbox = gtk::Box::new(Orientation::Vertical, 0);
        let image_outer = gtk::Box::new(Orientation::Vertical, 0);

        let label = Label::new(Some(constant::DEFAULT_INFORMATION));

        vbox.pack_end(&label, false, false, 0);
        vbox.pack_end(&image_outer, true, true, 0);
        window.add(&vbox);

        image_outer.show();
        vbox.show();
        window.show();

        Gui {
            cols: 0,
            rows: 0,
            window: window,
            images: vec![],
            image_outer: image_outer,
            image_inners: vec![],
            label: label,
        }
    }

    pub fn clear_images(&mut self) {
        for inner in &self.image_inners {
            self.image_outer.remove(inner);
        }
        self.cols = 0;
        self.rows = 0;
        self.images.clear();
        self.image_inners.clear();
    }

    pub fn reset_images(&mut self, cols: Option<usize>, rows: Option<usize>) {
        use gtk::Orientation;

        self.clear_images();

        if let Some(cols) = cols { self.cols = cols; }
        if let Some(rows) = rows { self.rows = rows; }

        for _ in 0..self.rows {
            let inner = gtk::Box::new(Orientation::Horizontal, 0);
            self.image_outer.pack_start(&inner, true, true, 0);
            for _ in 0..self.cols {
                let image = Image::new_from_pixbuf(None);
                image.show();
                inner.pack_start(&image, true, true, 0);
                self.images.push(image);
            }
            inner.show();
            self.image_inners.push(inner);
        }
    }

    pub fn get_image_size(&self, with_label: bool) -> (i32, i32) {
        let (width, height) = self.window.get_size();

        let width = width / self.cols as i32;
        let height = if with_label {
            (height / self.rows as i32) - self.label.get_allocated_height()
        } else {
            height / self.rows as i32
        };

        (width, height)
    }
}
