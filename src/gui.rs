
use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use cairo::{Context, ImageSurface, Format};
use gdk_pixbuf::{Pixbuf, PixbufAnimation};
use gtk::prelude::*;
use gtk::{self, Window, Image, Label, Orientation, ScrolledWindow, Adjustment};

use color::Color;
use constant;
use image_buffer;
use option::OptionValue;
use size::{FitTo, Size};
use state::ViewState;
use utils::feq;



#[derive(Clone)]
pub struct Gui {
    top_spacer: Image,
    bottom_spacer: Image,
    cell_outer: gtk::Box,
    cell_inners: Vec<CellInner>,
    pub colors: Colors,
    pub window: Window,
    pub label: Label,
}

#[derive(Clone)]
struct CellInner {
    container: gtk::Box,
    cells: Vec<Cell>,
}

#[derive(Clone)]
pub struct Cell {
    pub image: Image,
    pub window: ScrolledWindow,
}

pub struct CellIterator<'a> {
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

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down
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
            cell_outer: image_outer,
            cell_inners: vec![],
            label: label,
            colors: Colors::default()
        }
    }

    fn rows(&self) -> usize {
        self.cell_inners.len()
    }

    fn cols(&self) -> usize {
        self.cell_inners.first().unwrap().cells.len()
    }

    pub fn len(&self) -> usize {
        self.cols() * self.rows()
    }

    pub fn cells(&self, reverse: bool) -> CellIterator {
        CellIterator { gui: self, index: 0, reverse: reverse }
    }

    pub fn reset_view(&mut self, state: &ViewState) {
        self.clear_images();
        self.create_images(state);
    }

    pub fn reset_scrolls(&self, to_end: bool) {
        for cell in self.cells(false) {
            if let Some(adj) = cell.window.get_vadjustment() {
                adj.set_value(if to_end { adj.get_upper() } else { 0.0 });
                cell.window.set_vadjustment(&adj);
            }
            if let Some(adj) = cell.window.get_hadjustment() {
                adj.set_value(if to_end { adj.get_upper() } else { 0.0 });
                cell.window.set_hadjustment(&adj);
            }
        }
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
                self.window.override_background_color(self.window.get_state_flags(), &color.gdk_rgba()),
            StatusBar =>
                self.label.override_color(self.label.get_state_flags(), &color.gdk_rgba()),
            StatusBarBackground =>
                self.label.override_background_color(self.label.get_state_flags(), &color.gdk_rgba()),
            Error => self.colors.error = color.to_owned(),
            ErrorBackground => self.colors.error_background = color.to_owned(),
        }
    }

    pub fn scroll_views(&self, direction: &Direction, scroll_size: f64, count: usize) -> bool {
        let mut scrolled = false;
        for cell in self.cells(false) {
            scrolled |= scroll_window(&cell.window, direction, scroll_size, count);
        }
        scrolled
    }

    fn create_images(&mut self, state: &ViewState) {
        if state.center_alignment.is_enabled() {
            self.cell_outer.pack_start(&self.top_spacer, true, true, 0);
            self.top_spacer.show();
        } else {
            self.top_spacer.hide();
        }

        for _ in 0..state.rows {
            let mut cells = vec![];

            let inner = gtk::Box::new(Orientation::Horizontal, 0);

            if state.center_alignment.is_enabled() {
                let left_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&left_spacer, true, true, 0);
                left_spacer.show();
            }

            for _ in 0..state.cols {
                let scrolled = ScrolledWindow::new(None, None);
                let image = Image::new_from_pixbuf(None);
                scrolled.add_with_viewport(&image);
                scrolled.show();
                image.show();
                inner.pack_start(&scrolled, !state.center_alignment.is_enabled(), true, 0);
                cells.push(Cell::new(image, scrolled));
            }

            if state.center_alignment.is_enabled() {
                let right_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&right_spacer, true, true, 0);
                right_spacer.show();
            }

            self.cell_outer.pack_start(&inner, !state.center_alignment.is_enabled(), true, 0);
            inner.show();

            self.cell_inners.push(CellInner {
                container: inner,
                cells: cells
            });
        }

        if state.center_alignment.is_enabled() {
            self.cell_outer.pack_start(&self.bottom_spacer, true, true, 0);
            self.bottom_spacer.show();
        } else {
            self.bottom_spacer.hide();
        }
    }

    fn clear_images(&mut self) {
        for inner in &self.cell_inners {
            self.cell_outer.remove(&inner.container);
        }
        self.cell_outer.remove(&self.top_spacer);
        self.cell_outer.remove(&self.bottom_spacer);
        self.cell_inners.clear();
    }

    pub fn save<T: AsRef<Path>>(&self, path: &T, index: usize) -> Result<(), String> {
        self.cells(false).nth(index).ok_or_else(|| o!("Out of index")).and_then(|cell| {
            save_image(&cell.image, path)
        })
    }
}

impl Cell {
    pub fn new(image: Image, window: ScrolledWindow) -> Cell {
        Cell { image: image, window: window }
    }

    pub fn draw(&self, pixbuf: &Pixbuf, cell_size: &Size, fit_to: &FitTo) {
        self.image.set_from_pixbuf(Some(pixbuf));
        let (image_width, image_height) = (pixbuf.get_width(), pixbuf.get_height());
        let (ci_width, ci_height) = (min!(image_width, cell_size.width), min!(image_height, cell_size.height));
        match *fit_to {
            FitTo::Width =>
                self.window.set_size_request(cell_size.width, ci_height),
            FitTo::Height =>
                self.window.set_size_request(ci_width, cell_size.height),
            FitTo::Cell | FitTo::Original | FitTo::OriginalOrCell =>
                self.window.set_size_request(ci_width, ci_height),
        }
    }

    pub fn draw_animation(&self, pixbuf: &PixbufAnimation) {
        self.image.set_from_animation(pixbuf);
    }

    pub fn draw_error(&self, error: &image_buffer::Error, cell_size: &Size, fit_to: &FitTo, colors: &Colors) {
        self.draw(
            &error.get_pixbuf(cell_size, &colors.error, &colors.error_background),
            cell_size,
            fit_to);
    }

    /** return (x, y, w, h) **/
    pub fn get_top_left(&self) -> (i32, i32, i32, i32) {
        fn extract(adj: &Adjustment) -> (f64, f64) {
            (adj.get_value(), adj.get_upper())
        }

        let w = self.window.get_allocation();
        let (sx, sw) = self.window.get_hadjustment().as_ref().map(extract).unwrap();
        let (sy, sh) = self.window.get_vadjustment().as_ref().map(extract).unwrap();
        (w.x - sx as i32,
         w.y - sy as i32,
         sw as i32,
         sh as i32)
    }
}

impl<'a> Iterator for CellIterator<'a> {
    type Item = &'a Cell;

    fn next(&mut self) -> Option<&'a Cell> {
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
        let result = self.gui.cell_inners.get(rows).and_then(|inner| {
            inner.cells.get(cols)
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


impl FromStr for Direction {
    type Err = String;

    fn from_str(src: &str) -> Result<Direction, String> {
        use self::Direction::*;

        match src {
            "left" | "l" =>
                Ok(Left),
            "up" | "u" =>
                Ok(Up),
            "right" | "r" =>
                Ok(Right),
            "down" | "d" =>
                Ok(Down),
            _ =>
                Err(format!("Invalid direction: {}", src))
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

fn scroll_window(window: &ScrolledWindow, direction: &Direction, scroll_size_ratio: f64, count: usize) -> bool {
    use self::Direction::*;

    let scroll = |horizontal| -> bool {
        let adj = if horizontal { window.get_hadjustment() } else { window.get_vadjustment() };
        if let Some(adj) = adj {
            let scroll_size = adj.get_page_size() * scroll_size_ratio * count as f64;
            let scroll_size = match *direction {
                Right | Down => scroll_size,
                Left | Up => -scroll_size,
            };
            let value = adj.get_value();
            adj.set_value(value + scroll_size);
            if !feq(adj.get_value(), value, 0.0000001) {
                if horizontal { window.set_hadjustment(&adj) } else { window.set_vadjustment(&adj) }
                return true
            }
        }
        false
    };

    match *direction {
        Left | Right => scroll(true),
        Up | Down => scroll(false),
    }
}
