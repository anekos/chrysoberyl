
use std::str::FromStr;

use gdk_pixbuf::PixbufAnimationExt;
use gtk::prelude::*;
use gtk::{self, Window, Image, Label, Orientation};

use color::RGB;
use constant;



#[derive(Clone)]
pub struct Gui {
    cols: usize,
    rows: usize,
    image_outer: gtk::Box,
    image_inners: Vec<ImageInner>,
    pub colors: Colors,
    pub window: Window,
    pub label: Label,
}

#[derive(Clone)]
struct ImageInner {
    left_spacer: gtk::Label,
    right_spacer: gtk::Label,
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
            image_outer: image_outer,
            image_inners: vec![],
            label: label,
            colors: Colors::default()
        };

        result.create_images();

        result
    }

    pub fn len(&self) -> usize {
        self.cols * self.rows
    }

    pub fn images(&self) -> ImageIterator {
        ImageIterator { gui: self, index: 0 }
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
            Information =>
                self.label.override_color(self.label.get_state_flags(), &color.gdk_rgba()),
            InformationBackground =>
                self.label.override_background_color(self.label.get_state_flags(), &color.gdk_rgba()),
            Error => self.colors.error = color.to_owned(),
            ErrorBackground => self.colors.error_background = color.to_owned(),
        }
    }

    pub fn set_center_alignment(&mut self, enabled: bool) {
        let (window_width, _) = self.window.get_size();

        for inner in &self.image_inners {
            if enabled {
                let mut image_width_sum = 0;
                for image in &inner.images {
                    if let Some(width) = image.get_pixbuf().map(|it| it.get_width()).or_else(|| image.get_animation().map(|it| it.get_width())) {
                        image_width_sum += width;
                    }
                }
                let width = (window_width - image_width_sum) as u32 / 2 / 2;
                inner.container.set_child_packing(&inner.left_spacer, true, true, width, gtk::PackType::Start);
                inner.container.set_child_packing(&inner.right_spacer, true, true, width, gtk::PackType::Start);
                inner.left_spacer.show();
                inner.right_spacer.show();
            } else {
                inner.left_spacer.hide();
                inner.right_spacer.hide();
            }
        }
    }

    fn create_images(&mut self) {
        for _ in 0..self.rows {
            let left_spacer = gtk::Label::new(None);
            let right_spacer = gtk::Label::new(None);
            let mut images = vec![];

            let inner = gtk::Box::new(Orientation::Horizontal, 0);

            inner.pack_start(&left_spacer, true, true, 0);
            left_spacer.show();

            self.image_outer.pack_start(&inner, true, true, 0);

            for _ in 0..self.cols {
                let image = Image::new_from_pixbuf(None);
                image.show();
                inner.pack_start(&image, true, true, 0);
                images.push(image);
            }

            inner.pack_start(&right_spacer, true, true, 0);
            right_spacer.show();

            inner.show();
            self.image_inners.push(ImageInner {
                left_spacer: left_spacer,
                right_spacer: right_spacer,
                container: inner,
                images: images
            });
        }
    }

    fn clear_images(&mut self) {
        for inner in &self.image_inners {
            self.image_outer.remove(&inner.container);
        }
        // FIXME
        // self.images.clear();
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
            "information" | "information-fg" => Ok(Information),
            "information-background" | "information-bg" => Ok(InformationBackground),
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
            // information: RGB::new(0.0, 0.0, 0.0),
            // information_background: RGB::new(1.0, 1.0, 1.0),
            error: RGB::new(1.0, 1.0, 1.0),
            error_background: RGB::new(1.0, 0.0, 0.0),
        }
    }
}
