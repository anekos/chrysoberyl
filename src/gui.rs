
use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use cairo::{Context, ImageSurface, Format};
use css_color_parser::Color;
use gtk::prelude::*;
use gtk::{self, Window, Image, Label, Orientation};

use color::gdk_rgba;
use constant;
use size::Size;
use state::ViewState;



#[derive(Clone)]
pub struct Gui {
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
    index: usize,
    reverse: bool
}

#[derive(Clone, Debug, PartialEq)]
pub struct Colors {
    // pub window_background: RGB,
    // pub status_bar: RGB,
    // pub status_bar_background: RGB,
    pub error: Color,
    pub error_background: Color,
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
        window.set_role(constant::WINDOW_ROLE);
        window.set_border_width(0);
        window.set_position(gtk::WindowPosition::Center);
        window.set_wmclass(constant::WINDOW_CLASS, constant::WINDOW_CLASS);

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
            window: window,
            top_spacer: gtk::Image::new_from_pixbuf(None),
            bottom_spacer: gtk::Image::new_from_pixbuf(None),
            image_outer: image_outer,
            image_inners: vec![],
            label: label,
            colors: Colors::default()
        }
    }

    fn rows(&self) -> usize {
        self.image_inners.len()
    }

    fn cols(&self) -> usize {
        self.image_inners.first().unwrap().images.len()
    }

    pub fn len(&self) -> usize {
        self.cols() * self.rows()
    }

    pub fn images(&self, reverse: bool) -> ImageIterator {
        ImageIterator { gui: self, index: 0, reverse: reverse }
    }

    pub fn reset_view(&mut self, state: &ViewState) {
        self.clear_images();
        self.create_images(state);
    }

    pub fn get_cell_size(&self, state: &ViewState, with_label: bool) -> Size {
        let (width, height) = self.window.get_size();

        let width = width / state.cols as i32;
        let height = if with_label {
            (height / state.rows as i32) - self.label.get_allocated_height()
        } else {
            height / state.rows as i32
        };

        Size::new(width, height)
    }

    pub fn update_color(&mut self, target: &ColorTarget, color: &Color) {
        use self::ColorTarget::*;

        match *target {
            WindowBackground =>
                self.window.override_background_color(self.window.get_state_flags(), &gdk_rgba(color)),
            StatusBar =>
                self.label.override_color(self.label.get_state_flags(), &gdk_rgba(color)),
            StatusBarBackground =>
                self.label.override_background_color(self.label.get_state_flags(), &gdk_rgba(color)),
            Error => self.colors.error = color.to_owned(),
            ErrorBackground => self.colors.error_background = color.to_owned(),
        }
    }

    fn create_images(&mut self, state: &ViewState) {
        if state.center_alignment {
            self.image_outer.pack_start(&self.top_spacer, true, true, 0);
            self.top_spacer.show();
        } else {
            self.top_spacer.hide();
        }

        for _ in 0..state.rows {
            let mut images = vec![];

            let inner = gtk::Box::new(Orientation::Horizontal, 0);

            if state.center_alignment {
                let left_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&left_spacer, true, true, 0);
                left_spacer.show();
            }

            for _ in 0..state.cols {
                let image = Image::new_from_pixbuf(None);
                image.show();
                inner.pack_start(&image, !state.center_alignment, true, 0);
                images.push(image);
            }

            if state.center_alignment {
                let right_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&right_spacer, true, true, 0);
                right_spacer.show();
            }

            self.image_outer.pack_start(&inner, !state.center_alignment, true, 0);
            inner.show();

            self.image_inners.push(ImageInner {
                container: inner,
                images: images
            });
        }

        if state.center_alignment {
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

    pub fn save<T: AsRef<Path>>(&self, path: &T, index: usize) -> Result<(), String> {
        self.images(false).nth(index).ok_or_else(|| o!("Out of index")).and_then(|image| {
            save_image(image, path)
        })
    }
}

impl<'a> Iterator for ImageIterator<'a> {
    type Item = &'a Image;

    fn next(&mut self) -> Option<&'a Image> {
        let len = self.gui.len();
        let cols = self.gui.cols();
        let mut index = self.index;
        if self.reverse {
            if index < len {
                index = self.gui.len() - index - 1;
            } else {
                return None
            }
        }
        let rows = index / cols;
        let cols = index % cols;
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
            // window_background: "#004040",
            // status_bar: "#004040",
            // status_bar_background: "#004040",
            error: "white".parse().unwrap(),
            error_background: "red".parse().unwrap(),
        }
    }
}


fn save_image<T: AsRef<Path>>(image: &Image, path: &T) -> Result<(), String> {
    use gdk::prelude::ContextExt;

    image.get_pixbuf().ok_or_else(|| o!("No pixbuf")).and_then(|pixbuf| {
        let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
        let surface = ImageSurface::create(Format::ARgb32, width, height);
        let context = Context::new(&surface);
        context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
        context.paint();
        File::create(path).map_err(|it| s!(it)).and_then(|file| {
            surface.write_to_png(file).map_err(|_| o!("IO Error"))
        })
    })
}
