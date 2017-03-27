
use std::str::FromStr;

use gtk::prelude::*;
use gtk::{self, Window, Image, Label, Orientation};

use color::RGB;
use constant;



#[derive(Clone)]
pub struct Gui {
    cols: usize,
    rows: usize,
    image_outer: gtk::Box,
    image_inners: Vec<gtk::Box>,
    pub colors: Colors,
    pub window: Window,
    pub images: Vec<Image>,
    pub label: Label,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Colors {
    // pub window_background: RGB,
    // pub information: RGB,
    // pub information_background: RGB,
    pub error: RGB,
    pub error_background: RGB,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorTarget {
    WindowBackground,
    Information,
    InformationBackground,
    Error,
    ErrorBackground,
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

        let mut result = Gui {
            cols: 1,
            rows: 1,
            window: window,
            images: vec![],
            image_outer: image_outer,
            image_inners: vec![],
            label: label,
            colors: Colors::default()
        };

        result.create_images();

        result
    }

    pub fn reset_images(&mut self, cols: Option<usize>, rows: Option<usize>) -> bool {
        if (cols.is_none() || cols == Some(self.cols)) && (rows.is_none() || rows == Some(self.rows)) {
            return false;
        }

        self.clear_images();

        if let Some(cols) = cols { self.cols = cols; }
        if let Some(rows) = rows { self.rows = rows; }

        self.create_images();

        true
    }

    pub fn get_cell_size(&self, with_label: bool) -> (i32, i32) {
        let (width, height) = self.window.get_size();

        let width = width / self.cols as i32;
        let height = if with_label {
            (height / self.rows as i32) - self.label.get_allocated_height()
        } else {
            height / self.rows as i32
        };

        (width, height)
    }

    pub fn update_color(&mut self, target: &ColorTarget, color: &RGB) {
        use self::ColorTarget::*;

        match *target {
            WindowBackground =>
                self.window.override_background_color(self.window.get_state_flags(), &color.gdk_rgba()),
            // Information => (),
            // InformationBackground => (),
            Error => self.colors.error = color.to_owned(),
            ErrorBackground => self.colors.error_background = color.to_owned(),
            _ => puts_error("at" => "@color", "reason" => "Not implemented")
        }
    }

    fn create_images(&mut self) {
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

    fn clear_images(&mut self) {
        for inner in &self.image_inners {
            self.image_outer.remove(inner);
        }
        self.images.clear();
        self.image_inners.clear();
    }
}

impl FromStr for ColorTarget {
    type Err = String;

    fn from_str(src: &str) -> Result<ColorTarget, String> {
        use self::ColorTarget::*;

        match src {
            "window-background" => Ok(WindowBackground),
            "information" => Ok(Information),
            "information-background" => Ok(InformationBackground),
            "error" => Ok(Error),
            "error-background" => Ok(ErrorBackground),
            _ => Err(format!("Invalid name: {}", src))
        }
    }
}

impl Colors {
    pub fn default() -> Colors {
        Colors {
            // window_background: RGB::new(1.0, 1.0, 1.0),
            // information: RGB::new(0.0, 0.0, 0.0),
            // information_background: RGB::new(1.0, 1.0, 1.0),
            error: RGB::new(1.0, 1.0, 1.0),
            error_background: RGB::new(1.0, 0.0, 0.0),
        }
    }
}
