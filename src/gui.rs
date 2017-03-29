
use std::str::FromStr;

use gtk::prelude::*;
use gtk::{self, Window, Image, Label, Orientation};

use color::RGB;
use constant;



#[derive(Clone)]
pub struct Gui {
    cols: usize,
    rows: usize,
    center_alignment: bool,
    top_spacer: Image,
    bottom_spacer: Image,
    image_outer: gtk::Box,
    image_inners: Vec<ImageInner>,
    pub colors: Colors,
    pub window: Window,
    pub label: Label,
}

#[derive(Clone)]
struct ImageInner {
    container: gtk::Box,
    images: Vec<Image>,
}

pub struct ImageIterator<'a> {
    gui: &'a Gui,
    index: usize
}

#[derive(Clone, Debug, PartialEq)]
pub struct Colors {
    // pub window_background: RGB,
    // pub status_bar: RGB,
    // pub status_bar_background: RGB,
    pub error: RGB,
    pub error_background: RGB,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ColorTarget {
    WindowBackground,
    StatusBar,
    StatusBarBackground,
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
            center_alignment: false,
            window: window,
            top_spacer: gtk::Image::new_from_pixbuf(None),
            bottom_spacer: gtk::Image::new_from_pixbuf(None),
            image_outer: image_outer,
            image_inners: vec![],
            label: label,
            colors: Colors::default()
        };

        result.create_images(false);

        result
    }

    pub fn len(&self) -> usize {
        self.cols * self.rows
    }

    pub fn images(&self) -> ImageIterator {
        ImageIterator { gui: self, index: 0 }
    }

    pub fn reset_images(&mut self, cols: Option<usize>, rows: Option<usize>, center_alignment: bool) -> bool {
        if (cols.is_none() || cols == Some(self.cols)) && (rows.is_none() || rows == Some(self.rows)) && (center_alignment == self.center_alignment) {
            return false;
        }

        self.clear_images();

        if let Some(cols) = cols { self.cols = cols; }
        if let Some(rows) = rows { self.rows = rows; }
        self.center_alignment = center_alignment;

        self.create_images(center_alignment);

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
            StatusBar =>
                self.label.override_color(self.label.get_state_flags(), &color.gdk_rgba()),
            StatusBarBackground =>
                self.label.override_background_color(self.label.get_state_flags(), &color.gdk_rgba()),
            Error => self.colors.error = color.to_owned(),
            ErrorBackground => self.colors.error_background = color.to_owned(),
        }
    }

    fn create_images(&mut self, center_alignment: bool) {
        if center_alignment {
            self.image_outer.pack_start(&self.top_spacer, true, true, 0);
            self.top_spacer.show();
        } else {
            self.top_spacer.hide();
        }

        for _ in 0..self.rows {
            let mut images = vec![];

            let inner = gtk::Box::new(Orientation::Horizontal, 0);

            if center_alignment {
                let left_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&left_spacer, true, true, 0);
                left_spacer.show();
            }

            for _ in 0..self.cols {
                let image = Image::new_from_pixbuf(None);
                image.show();
                inner.pack_start(&image, !center_alignment, true, 0);
                images.push(image);
            }

            if center_alignment {
                let right_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&right_spacer, true, true, 0);
                right_spacer.show();
            }

            self.image_outer.pack_start(&inner, !center_alignment, true, 0);
            inner.show();

            self.image_inners.push(ImageInner {
                container: inner,
                images: images
            });
        }

        if center_alignment {
            self.image_outer.pack_start(&self.bottom_spacer, true, true, 0);
            self.bottom_spacer.show();
        } else {
            self.bottom_spacer.hide();
        }
    }

    fn clear_images(&mut self) {
        for inner in &self.image_inners {
            self.image_outer.remove(&inner.container);
        }
        self.image_outer.remove(&self.top_spacer);
        self.image_outer.remove(&self.bottom_spacer);
        self.image_inners.clear();
    }
}

impl<'a> Iterator for ImageIterator<'a> {
    type Item = &'a Image;

    fn next(&mut self) -> Option<&'a Image> {
        let rows = self.index / self.gui.cols;
        let cols = self.index % self.gui.cols;
        let result = self.gui.image_inners.get(rows).and_then(|inner| {
            inner.images.get(cols)
        });
        self.index += 1;
        result
    }
}

impl FromStr for ColorTarget {
    type Err = String;

    fn from_str(src: &str) -> Result<ColorTarget, String> {
        use self::ColorTarget::*;

        match src {
            "window-background" | "window-bg" => Ok(WindowBackground),
            "status-bar" | "status-bar-fg" => Ok(StatusBar),
            "status-bar-background" | "status-bar-bg" => Ok(StatusBarBackground),
            "error" | "error-fg" => Ok(Error),
            "error-background" | "error-bg" => Ok(ErrorBackground),
            _ => Err(format!("Invalid name: {}", src))
        }
    }
}

impl Colors {
    pub fn default() -> Colors {
        Colors {
            // window_background: RGB::new(1.0, 1.0, 1.0),
            // status_bar: RGB::new(0.0, 0.0, 0.0),
            // status_bar_background: RGB::new(1.0, 1.0, 1.0),
            error: RGB::new(1.0, 1.0, 1.0),
            error_background: RGB::new(1.0, 0.0, 0.0),
        }
    }
}
