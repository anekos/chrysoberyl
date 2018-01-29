
use std::convert::Into;
use std::default::Default;
use std::fs::File;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::Sender;

use cairo::{Context, ImageSurface, Format};
use gdk::EventMask;
use gdk_pixbuf::{Pixbuf, PixbufAnimationExt};
use gtk::prelude::*;
use gtk::{self, Window, Image, Label, Orientation, ScrolledWindow, Adjustment, Entry, Overlay, TextView, TextBuffer};

use color::Color;
use constant;
use errors::*;
use gtk_utils::new_pixbuf_from_surface;
use image::{ImageBuffer, StaticImageBuffer, AnimationBuffer};
use operation::Operation;
use size::{FitTo, Size, Region};
use state::ViewState;
use ui_event::UIEvent;
use util::num::feq;



enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    pub enum DropItemType {
        Path = 0,
        URI = 1,
    }
}


pub struct Gui {
    top_spacer: Image,
    bottom_spacer: Image,
    cell_outer: gtk::Box,
    cell_inners: Vec<CellInner>,
    operation_box: gtk::Box,
    log_buffer: TextBuffer,
    ui_event: Option<UIEvent>,
    pub colors: Colors,
    pub window: Window,
    pub vbox: gtk::Box,
    pub label: Label,
    pub operation_entry: Entry,
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
    pub window_background: Color,
    pub status_bar: Color,
    pub status_bar_background: Color,
    pub error: Color,
    pub error_background: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
    Left,
    Up,
    Right,
    Down
}


const FONT_SIZE: f64 = 12.0;
const PADDING: f64 = 5.0;


impl Gui {
    pub fn new(window_role: &str) -> Gui {
        use gtk::Orientation;
        use gdk::DragAction;
        use gtk::{DestDefaults, TargetEntry, TargetFlags};

        gtk::init().unwrap();

        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        window.set_title(constant::DEFAULT_TITLE);
        window.set_role(window_role);
        window.set_border_width(0);
        window.set_position(gtk::WindowPosition::Center);
        window.set_wmclass(constant::WINDOW_CLASS, constant::WINDOW_CLASS);
        window.add_events(EventMask::SCROLL_MASK.bits() as i32);

        let overlay = Overlay::new();
        let vbox = gtk::Box::new(Orientation::Vertical, 0);
        let cell_outer = gtk::Box::new(Orientation::Vertical, 0);
        let operation_box = gtk::Box::new(Orientation::Vertical, 0);
        let label = Label::new(None);

        vbox.add_events(EventMask::SCROLL_MASK.bits() as i32);

        {
            let action = DragAction::COPY | DragAction::MOVE | DragAction::DEFAULT | DragAction::LINK | DragAction::ASK | DragAction::PRIVATE;
            let flags = TargetFlags::OTHER_WIDGET | TargetFlags::OTHER_APP;
            let targets = vec![
                TargetEntry::new("text/uri-list", flags, DropItemType::Path.into()),
                TargetEntry::new("text/plain", flags, DropItemType::URI.into()),

                // TargetEntry::new("text/html", flags, 0),
                // TargetEntry::new("text/x-moz-url", flags, 0),

                // TargetEntry::new("application/x-moz-file", flags, 0),
                // TargetEntry::new("text/unicode", flags, 0),
                // TargetEntry::new("text/plain;charset=utf-8", flags, 0),
                // TargetEntry::new("application/x-moz-custom-clipdata", flags, 0),
                // TargetEntry::new("text/_moz_htmlcontext", flags, 0),
                // TargetEntry::new("text/_moz_htmlinfo", flags, 0),
                // TargetEntry::new("_NETSCAPE_URL", flags, 0),
                // TargetEntry::new("text/x-moz-url-data", flags, 0),
                // TargetEntry::new("text/x-moz-url-desc", flags, 0),
                // TargetEntry::new("application/x-moz-nativeimage", flags, 0),
                // TargetEntry::new("application/x-moz-file-promise", flags, 0),
                // TargetEntry::new("application/x-moz-file-promise-url", flags, 0),
                // TargetEntry::new("application/x-moz-file-promise-dest-filename", flags, 0),
            ];
            vbox.drag_dest_set(DestDefaults::ALL, &targets, action);
        }

        let operation_entry = Entry::new();
        operation_entry.set_text("");
        let log_scrolled = ScrolledWindow::new(None, None);
        let log_buffer = TextBuffer::new(None);
        let log_view = TextView::new_with_buffer(&log_buffer);
        log_scrolled.add_with_viewport(&log_view);
        operation_box.pack_end(&operation_entry, false, false, 0);
        operation_box.pack_end(&log_scrolled, false, false, 0);

        vbox.pack_end(&label, false, false, 0);
        vbox.pack_end(&cell_outer, true, true, 0);

        overlay.add_overlay(&vbox);
        overlay.add_overlay(&operation_box);

        window.add(&overlay);
        overlay.show_all();
        operation_box.hide();

        Gui {
            bottom_spacer: gtk::Image::new_from_pixbuf(None),
            cell_inners: vec![],
            cell_outer,
            colors: Colors::default(),
            label,
            log_buffer,
            operation_box,
            operation_entry,
            top_spacer: gtk::Image::new_from_pixbuf(None),
            ui_event: None,
            vbox,
            window,
        }
    }

    pub fn show(&self) {
        self.window.show();
    }

    pub fn register_ui_events(&mut self, skip: usize, app_tx: &Sender<Operation>) {
        self.ui_event = Some(UIEvent::new(self, skip, app_tx));
    }

    /**
     * if visibility is updated, returns true.
     */
    pub fn set_operation_box_visibility(&self, visibility: bool) {
        use gtk::DirectionType::*;

        let current = self.operation_box.get_visible();
        if visibility ^ current {
            if let Some(ref ui_event) = self.ui_event {
                ui_event.update_entry(visibility);
            }

            if visibility {
                self.operation_entry.set_text("");
                self.operation_box.show();
                self.operation_entry.grab_focus();
                self.window.set_events(0);
            } else {
                self.operation_box.hide();
                self.window.child_focus(Down); // To blur
            }
        }
    }

    pub fn rows(&self) -> usize {
        self.cell_inners.len()
    }

    pub fn cols(&self) -> usize {
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

    pub fn make_visibles(&self, regions: &[Option<Region>]) {
        for (cell, region) in self.cells(false).zip(regions) {
            if let Some(ref region) = *region {
                cell.make_visible(region);
            }
        }
    }

    pub fn get_cell_size(&self, state: &ViewState) -> Size {
        let (width, height) = self.window.get_size();
        let label_height = if self.label.get_visible() { self.label.get_allocated_height() } else { 0 };

        let width = width / state.cols as i32;
        let height = height - label_height;
        let height = height / state.rows as i32;

        Size::new(width, height)
    }

    pub fn update_colors(&self) {
        self.window.override_background_color(
            self.window.get_state_flags(),
            &self.colors.window_background.gdk_rgba());
        self.label.override_color(
            self.label.get_state_flags(),
            &self.colors.status_bar.gdk_rgba());
        self.label.override_background_color(
            self.label.get_state_flags(),
            &self.colors.status_bar_background.gdk_rgba());
    }

    pub fn scroll_views(&self, direction: &Direction, scroll_size: f64, count: usize, crush: bool) -> bool {
        let mut scrolled = false;
        for cell in self.cells(false) {
            scrolled |= scroll_window(&cell.window, direction, scroll_size, count, crush);
        }
        scrolled
    }

    pub fn log(&self, line: &str) {
        let mut iter = self.log_buffer.get_end_iter();
        self.log_buffer.insert(&mut iter, line);
        self.log_buffer.insert(&mut iter, "\n");
    }

    fn create_images(&mut self, state: &ViewState) {
        if state.center_alignment {
            self.cell_outer.pack_start(&self.top_spacer, true, true, 0);
            self.top_spacer.show();
        } else {
            self.top_spacer.hide();
        }

        for _ in 0..state.rows {
            let mut cells = vec![];

            let inner = gtk::Box::new(Orientation::Horizontal, 0);

            if state.center_alignment {
                let left_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&left_spacer, true, true, 0);
                left_spacer.show();
            }

            for _ in 0..state.cols {
                let scrolled = ScrolledWindow::new(None, None);
                let image = Image::new_from_pixbuf(None);
                scrolled.connect_button_press_event(|_, _| Inhibit(true));
                scrolled.connect_button_release_event(|_, _| Inhibit(true));
                scrolled.connect_scroll_event(|_, _| Inhibit(true));
                scrolled.add_with_viewport(&image);
                scrolled.show();
                image.show();
                inner.pack_start(&scrolled, !state.center_alignment, true, 0);
                cells.push(Cell::new(image, scrolled));
            }

            if state.center_alignment {
                let right_spacer = gtk::Image::new_from_pixbuf(None);
                inner.pack_start(&right_spacer, true, true, 0);
                right_spacer.show();
            }

            self.cell_outer.pack_start(&inner, !state.center_alignment, true, 0);
            inner.show();

            self.cell_inners.push(CellInner {
                container: inner,
                cells: cells
            });
        }

        if state.center_alignment {
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

    pub fn save<T: AsRef<Path>>(&self, path: &T, index: usize) -> Result<(), BoxedError> {
        let cell = self.cells(false).nth(index).ok_or("Out of index")?;
        save_image(&cell.image, path)
    }
}

impl Cell {
    pub fn new(image: Image, window: ScrolledWindow) -> Cell {
        Cell { image: image, window: window }
    }

    /**
     * @return Scale
     */
    pub fn draw(&self, image_buffer: &ImageBuffer, cell_size: &Size, fit_to: &FitTo, fg: &Color, bg: &Color) -> Option<f64> {
        match *image_buffer {
            ImageBuffer::Static(ref buf) =>
                self.draw_static(buf, cell_size, fit_to),
            ImageBuffer::Animation(ref buf) => {
                self.draw_animation(buf, cell_size, fg, bg);
                None
            }
        }
    }

    pub fn draw_text(&self, text: &str, cell_size: &Size, fg: &Color, bg: &Color) {
        let surface = ImageSurface::create(Format::ARgb32, cell_size.width, cell_size.height).unwrap();

        let (width, height) = cell_size.floated();

        let context = Context::new(&surface);

        context.set_font_size(FONT_SIZE);
        let extents = context.text_extents(text);

        let (x, y) = (width / 2.0 - extents.width / 2.0, height / 2.0 - extents.height / 2.0);

        let bg = bg.gdk_rgba();
        context.set_source_rgba(bg.red, bg.green, bg.blue, bg.alpha);
        context.rectangle(
            x - PADDING,
            y - extents.height - PADDING,
            extents.width + PADDING * 2.0,
            extents.height + PADDING * 2.0);
        context.fill();

        context.move_to(x, y);
        let fg = fg.gdk_rgba();
        context.set_source_rgba(fg.red, fg.green, fg.blue, fg.alpha);
        context.show_text(text);

        // puts_error!("at" => "show_image", "reason" => text);

        self.draw_pixbuf(&new_pixbuf_from_surface(&surface), cell_size, &FitTo::Original);
    }

    fn draw_static(&self, image_buffer: &StaticImageBuffer, cell_size: &Size, fit_to: &FitTo) -> Option<f64> {
        self.draw_pixbuf(&image_buffer.get_pixbuf(), cell_size, fit_to);
        image_buffer.original_size.map(|original_size| {
            original_size.fit(cell_size, fit_to).0
        })
    }

    fn draw_pixbuf(&self, pixbuf: &Pixbuf, cell_size: &Size, fit_to: &FitTo) {
        use size::FitTo::*;

        self.image.set_from_pixbuf(Some(pixbuf));
        let image_size = Size::new(pixbuf.get_width(), pixbuf.get_height());
        let (ci_width, ci_height) = (min!(image_size.width, cell_size.width), min!(image_size.height, cell_size.height));
        match *fit_to {
            Width =>
                self.window.set_size_request(cell_size.width, ci_height),
            Height =>
                self.window.set_size_request(ci_width, cell_size.height),
            Cell | Original | OriginalOrCell | Fixed(_, _) | Scale(_) =>
                self.window.set_size_request(ci_width, ci_height),
        }
    }

    fn draw_animation(&self, image_buffer: &AnimationBuffer, cell_size: &Size, fg: &Color, bg: &Color) {
        match image_buffer.get_pixbuf_animation() {
            Ok(buf) => {
                self.image.set_from_animation(&buf);
                let (w, h) = (buf.get_width(), buf.get_height());
                self.window.set_size_request(w, h);
            }
            Err(ref error) =>
                self.draw_text(&s!(error), cell_size, fg, bg)
        }
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

    pub fn get_image_size(&self) -> Option<(i32, i32)> {
        self.image.get_pixbuf()
            .map(|it| (it.get_width(), it.get_height()))
            .or_else(|| {
                self.image.get_animation()
                    .map(|it| (it.get_width(), it.get_height()))
            })
    }

    pub fn make_visible(&self, region: &Region) {
        let (h_center, v_center) = region.centroids();

        if let Some(adj) = self.window.get_hadjustment() {
            let (width, page_width) = (adj.get_upper(), adj.get_page_size());
            adj.set_value(h_center * width - page_width / 2.0);
            self.window.set_hadjustment(&adj);
        }

        if let Some(adj) = self.window.get_vadjustment() {
            let (height, page_height) = (adj.get_upper(), adj.get_page_size());
            adj.set_value(v_center * height - page_height / 2.0);
            self.window.set_vadjustment(&adj);
        }
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


impl Default for Colors {
    fn default() -> Colors {
        Colors {
            window_background: "#004040".parse().unwrap(),
            status_bar: "white".parse().unwrap(),
            status_bar_background: "#004040".parse().unwrap(),
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


impl Into<u32> for DropItemType {
    fn into(self) -> u32 {
        self as u32
    }
}


fn save_image<T: AsRef<Path>>(image: &Image, path: &T) -> Result<(), BoxedError> {
    use gdk::prelude::ContextExt;

    let pixbuf = image.get_pixbuf().ok_or("No pixbuf")?;
    let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let context = Context::new(&surface);
    context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
    context.paint();
    let mut file = File::create(path)?;
    surface.write_to_png(&mut file).map_err(ChryError::from)?;
    Ok(())
}

fn scroll_window(window: &ScrolledWindow, direction: &Direction, scroll_size_ratio: f64, count: usize, crush: bool) -> bool {
    use self::Direction::*;

    let scroll = |horizontal| -> bool {
        let adj = if horizontal { window.get_hadjustment() } else { window.get_vadjustment() };
        if let Some(adj) = adj {
            let page_size = adj.get_page_size();
            let scroll_size = page_size * scroll_size_ratio * count as f64;
            let space = page_size * (1.0 - scroll_size_ratio);
            let value = adj.get_value();
            let scroll_size = match *direction {
                Right | Down => {
                    let rest = adj.get_upper() - value - scroll_size - page_size;
                    if rest < space && crush {
                        scroll_size + rest
                    } else {
                        scroll_size
                    }
                }
                Left | Up => {
                    let rest = value - scroll_size;
                    if rest < space && crush {
                        -(scroll_size + rest)
                    } else {
                        -scroll_size
                    }
                }
            };

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
