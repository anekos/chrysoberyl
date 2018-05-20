
use std::convert::Into;
use std::default::Default;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::ops;
use std::path::Path;
use std::str::FromStr;
use std::sync::mpsc::Sender;

use cairo::{Context, ImageSurface, Format};
use gdk::EventMask;
use gdk_pixbuf::{Pixbuf, PixbufExt, PixbufAnimationExt};
use glib::Type;
use gtk::prelude::*;
use gtk::{Adjustment, Align, Entry, EntryCompletion, Grid, Image, Label, Layout, ListStore, Overlay, ScrolledWindow, self, Value, Window};

use app_path;
use color::Color;
use constant;
use errors::*;
use gtk_utils::new_pixbuf_from_surface;
use image::{ImageBuffer, StaticImageBuffer, AnimationBuffer};
use operation::Operation;
use size::{Coord, CoordPx, FitTo, Region, Size};
use state::DrawingState;
use ui_event::UIEvent;
use util::num::feq;
use util;



enum_from_primitive! {
    #[derive(Debug, PartialEq)]
    pub enum DropItemType {
        Path = 0,
        URI = 1,
    }
}


pub struct Gui {
    pub colors: Colors,
    pub operation_entry: Entry,
    pub overlay: Overlay,
    pub vbox: gtk::Box,
    pub window: Window,
    cells: Vec<Cell>,
    entry_history: ListStore,
    grid: Grid,
    grid_size: Size,
    hidden_label: Label,
    label: Label,
    operation_box: gtk::Box,
    status_bar: Layout,
    status_bar_inner: gtk::Box,
    ui_event: Option<UIEvent>,
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

pub struct Views {
    pub cols: usize,
    pub rows: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum Position {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

const FONT_SIZE: f64 = 12.0;
const PADDING: f64 = 5.0;


impl Gui {
    pub fn new(window_role: &str) -> Gui {
        use gtk::Orientation;

        gtk::init().unwrap();

        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        window.set_title(constant::DEFAULT_TITLE);
        window.set_role(window_role);
        window.set_border_width(0);
        window.set_position(gtk::WindowPosition::Center);
        window.add_events(EventMask::SCROLL_MASK.bits() as i32);

        let grid = tap!(it = gtk::Grid::new(), {
            it.set_halign(Align::Center);
            it.set_valign(Align::Center);
            it.set_row_spacing(0);
            it.set_column_spacing(0);
        });
        let cells = vec![];

        let label = tap!(it = Label::new(None), {
            it.set_halign(Align::Center);
        });

        let hidden_label = Label::new("HIDDEN");

        let hidden_bar_inner = tap!(it = gtk::Box::new(Orientation::Vertical, 0), {
            it.pack_end(&hidden_label, true, true, 0);
            it.set_margin_top(20_000);
        });

        let hidden_bar = tap!(it = Layout::new(None, None), {
            it.add(&hidden_bar_inner);
        });

        let status_bar_inner = tap!(it = gtk::Box::new(Orientation::Vertical, 0), {
            it.pack_end(&label, true, true, 0);
            it.set_margin_top(2); // FIXME Magical number to remove bottom blank space.
        });

        let status_bar = tap!(it = Layout::new(None, None), {
            it.add(&status_bar_inner);
        });

        let entry_history = tap!(it = ListStore::new(&[Type::String]), {
            let _ = load_initial_completion(&it);
        });

        let operation_entry = tap!(it = Entry::new(), {
            it.set_text("");
            it.set_completion(&tap!(it = EntryCompletion::new(), {
                it.set_model(&entry_history);
                it.set_text_column(0);
                it.set_inline_completion(true);
                it.set_inline_selection(true);
                it.set_popup_single_match(false);
                it.set_popup_completion(true);
                it.set_minimum_key_length(1);
            }));
            it.connect_key_press_event(|_, key| {
                use ::gdk::enums::key::*;
                let key = key.as_ref().keyval;
                Inhibit(key == Tab || key == ISO_Left_Tab)
            });
        });

        let operation_box = tap!(it = gtk::Box::new(Orientation::Vertical, 0), {
            it.pack_end(&operation_entry, false, false, 0);
        });

        let vbox = tap!(it = gtk::Box::new(Orientation::Vertical, 0), {
            it.pack_end(&status_bar, false, false, 0);
            it.pack_end(&grid, true, true, 0);
        });

        let overlay = tap!(it = Overlay::new(), {
            setup_drag(&it);
            it.add_overlay(&vbox);
            it.add_overlay(&hidden_bar);
            it.add_overlay(&operation_box);
            it.show_all();
        });

        window.add(&overlay);

        Gui {
            cells,
            colors: Colors::default(),
            entry_history,
            grid,
            grid_size: Size::new(1, 1),
            hidden_label,
            label,
            operation_box,
            operation_entry,
            overlay,
            status_bar,
            status_bar_inner,
            ui_event: None,
            vbox,
            window,
        }
    }

    pub fn cells(&self, reverse: bool) -> CellIterator {
        CellIterator { gui: self, index: 0, reverse }
    }

    pub fn cols(&self) -> usize {
        self.grid_size.width as usize
    }

    pub fn get_cell_size(&self, state: &Views) -> Size {
        let (width, height) = self.window.get_size();
        let status_bar_height = if self.status_bar.get_visible() { self.status_bar.get_allocated_height() } else { 0 };

        let width = width / state.cols as i32;
        let height = height - status_bar_height;
        let height = height / state.rows as i32;

        Size::new(width, height)
    }

    pub fn len(&self) -> usize {
        self.cols() * self.rows()
    }

    pub fn make_visibles(&self, regions: &[Option<Region>]) {
        for (cell, region) in self.cells(false).zip(regions) {
            if let Some(ref region) = *region {
                cell.make_visible(region);
            }
        }
    }

    pub fn pop_operation_entry(&self) -> Result<Option<Operation>, Box<Error>> {
        if_let_some!(result = self.operation_entry.get_text(), Ok(None));
        if result.is_empty() {
            return Ok(None);
        }

        self.operation_entry.set_text("");

        let op = Operation::parse_fuzziness(&result);
        if op.is_ok() {
            append_completion_entry(&self.entry_history, &result, true);
        }

        Ok(Some(op?))
    }

    pub fn refresh_status_bar_width(&self) {
        let width = self.vbox.get_allocated_width();
        self.status_bar_inner.set_property_width_request(width);
    }

    pub fn register_ui_events(&mut self, skip: usize, app_tx: &Sender<Operation>) {
        self.ui_event = Some(UIEvent::new(self, skip, app_tx));
    }

    pub fn reset_scrolls(&self, position: Position, to_end: bool) {
        for cell in self.cells(false) {
            cell.reset_scroll(position, to_end);
        }
    }

    pub fn reset_view(&mut self, state: &Views) {
        self.create_images(state);
        self.reset_focus();
    }

    pub fn rows(&self) -> usize {
        self.grid_size.height as usize
    }

    pub fn save<T: AsRef<Path>>(&self, path: &T, index: usize) -> Result<(), BoxedError> {
        let cell = self.cells(false).nth(index).ok_or("Out of index")?;
        save_image(&cell.image, path)
    }
    pub fn scroll_views(&self, direction: &Direction, scroll_size: f64, crush: bool, reset_at_end: bool, count: usize) -> bool {
        let mut scrolled = false;
        for cell in self.cells(false) {
            scrolled |= scroll_window(&cell.window, direction, scroll_size, crush, reset_at_end, count);
        }
        scrolled
    }

    pub fn set_operation_box_visibility(&self, visibility: bool) {
        let current = self.operation_box.get_visible();
        if visibility ^ current {
            if let Some(ref ui_event) = self.ui_event {
                ui_event.update_entry(visibility);
            }

            if visibility {
                self.operation_entry.grab_focus();
                self.operation_box.show();
            } else {
                self.operation_box.hide();
                self.reset_focus();
            }
        }
    }

    pub fn set_status_bar_align(&self, align: Align) {
        self.label.set_halign(align);
    }

    pub fn set_status_bar_height(&self, height: Option<usize>) {
        let height = if let Some(height) = height {
            height as i32
        } else {
            self.hidden_label.get_allocated_height() * 110 / 100
        };
        self.status_bar.set_property_height_request(height);
    }

    pub fn set_status_bar_markup(&self, markup: &str) {
        self.label.set_markup(markup);
        self.hidden_label.set_markup(markup);
    }

    pub fn set_status_bar_visibility(&self, visibility: bool) {
        if visibility {
            self.status_bar.show_all();
        } else {
            self.status_bar.hide();
        }
    }

    pub fn show(&self) {
        self.window.show();
    }

    pub fn update_colors(&self) {
        self.window.override_background_color(
            self.window.get_state_flags(),
            &self.colors.window_background.gdk_rgba());
        self.label.override_color(
            self.label.get_state_flags(),
            &self.colors.status_bar.gdk_rgba());
        self.status_bar_inner.override_background_color(
            self.status_bar_inner.get_state_flags(),
            &self.colors.status_bar_background.gdk_rgba());
    }

    fn create_images(&mut self, state: &Views) {
        for cell in &self.cells {
            cell.image.set_from_pixbuf(None);
        }
        for child in &self.grid.get_children() {
            self.grid.remove(child);
        }
        self.cells.clear();

        for row in 0..state.rows {
            for col in 0..state.cols {
                let scrolled = ScrolledWindow::new(None, None);
                let image = Image::new_from_pixbuf(None);
                scrolled.connect_button_press_event(|_, _| Inhibit(true));
                scrolled.connect_button_release_event(|_, _| Inhibit(true));
                scrolled.connect_scroll_event(|_, _| Inhibit(true));
                scrolled.add(&image);
                scrolled.show();
                image.show();
                self.grid.attach(&scrolled, col as i32, row as i32, 1, 1);
                self.cells.push(Cell { image, window: scrolled });
            }
        }

        self.grid_size = Size::new(state.cols as i32, state.rows as i32);
        self.reset_focus();
    }

    fn reset_focus(&self) {
        if !self.window.get_visible() {
            return;
        }

        if self.operation_box.get_visible() {
            self.window.set_focus(Some(&self.operation_entry));
            return
        }

        if let Some(cell) = self.cells.first() {
            self.window.set_focus(Some(&cell.window));
        }
    }
}

impl Cell {
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

    pub fn get_image_size(&self) -> Option<(i32, i32)> {
        self.image.get_pixbuf()
            .map(|it| (it.get_width(), it.get_height()))
            .or_else(|| {
                self.image.get_animation()
                    .map(|it| (it.get_width(), it.get_height()))
            })
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

    pub fn get_position_on_image(&self, coord: &CoordPx, drawing: &DrawingState) -> Option<(Coord)> {
        fn extract(adj: &Adjustment) -> (f64, f64) {
            (adj.get_value(), adj.get_upper())
        }

        let a = self.window.get_allocation();

        if !(a.x <= coord.x && coord.x <= a.x + a.width && a.y <= coord.y && coord.y <= a.y + a.height) {
            return None;
        }

        let (px, py) = map!(f64, coord.x, coord.y);

        let (cx, cy) = map!(f64, a.x, a.y);

        let (sx, sw) = self.window.get_hadjustment().as_ref().map(extract).unwrap();
        let (sy, sh) = self.window.get_vadjustment().as_ref().map(extract).unwrap();
        let (sx, sy, sw, sh) = map!(f64, sx, sy, sw, sh);

        let (ix, iy) = (px - cx + sx, py - cy + sy);
        let (mut rx, mut ry) = (ix / sw, iy / sh);

        if let Some(clipping) = drawing.clipping.as_ref() {
            rx = rx * clipping.width() + clipping.left;
            ry = ry * clipping.height() + clipping.top;
        }

        let (rx, ry) = match drawing.rotation % 4 {
            1 => (ry, 1.0 - rx),
            2 => (1.0 - rx, 1.0 - ry),
            3 => (1.0 - ry, rx),
            _ => (rx, ry),
        };

        // println!("i: {}x{}, p: {}x{}, s: {}x{}-{}x{}, c: {}x{}, r: {}x{}", ix, iy, px, py, sx, sy, sw, sh, cx, cy, rx, ry);

        if 0.0 <= rx && 0.0 <= ry {
            Some(Coord { x: rx, y: ry })
        } else {
            None
        }
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

    fn draw_animation(&self, image_buffer: &AnimationBuffer, cell_size: &Size, fg: &Color, bg: &Color) {
        match image_buffer.get_pixbuf_animation() {
            Ok(buf) => {
                self.image.set_from_animation(&buf);
                let (ci_width, ci_height) = (min!(buf.get_width(), cell_size.width), min!(buf.get_height(), cell_size.height));
                self.window.set_size_request(ci_width, ci_height);
            }
            Err(ref error) =>
                self.draw_text(&s!(error), cell_size, fg, bg)
        }
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

    fn draw_static(&self, image_buffer: &StaticImageBuffer, cell_size: &Size, fit_to: &FitTo) -> Option<f64> {
        self.draw_pixbuf(&image_buffer.get_pixbuf(), cell_size, fit_to);
        image_buffer.original_size.map(|original_size| {
            original_size.fit(cell_size, fit_to).0
        })
    }

    fn reset_scroll(&self, position: Position, to_end: bool) {
        use self::Position::*;

        if_let_some!(h_adj = self.window.get_hadjustment(), ());
        if_let_some!(v_adj = self.window.get_vadjustment(), ());

        let (h_upper, v_upper) = (h_adj.get_upper(), v_adj.get_upper());

        let position = if to_end { !position } else { position };
        let (h, v) = match position {
            TopLeft => (0.0, 0.0),
            TopRight => (h_upper, 0.0),
            BottomLeft => (0.0, v_upper),
            BottomRight => (h_upper, v_upper),
        };

        v_adj.set_value(v);
        h_adj.set_value(h);

        self.window.set_vadjustment(&v_adj);
        self.window.set_hadjustment(&h_adj);
    }
}


impl<'a> Iterator for CellIterator<'a> {
    type Item = &'a Cell;

    fn next(&mut self) -> Option<&'a Cell> {
        let mut index = self.index;
        if self.reverse {
            if index < self.gui.len() {
                index = self.gui.len() - index - 1;
            } else {
                return None
            }
        }
        tap!(it = self.gui.cells.get(index), {
            self.index += 1;
        })
    }
}


impl Default for Colors {
    fn default() -> Colors {
        Colors {
            error: "white".parse().unwrap(),
            error_background: "red".parse().unwrap(),
            status_bar: "white".parse().unwrap(),
            status_bar_background: "#004040".parse().unwrap(),
            window_background: "#004040".parse().unwrap(),
        }
    }
}


impl FromStr for Direction {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Direction, ChryError> {
        use self::Direction::*;

        match src {
            "down" | "d" =>
                Ok(Down),
            "left" | "l" =>
                Ok(Left),
            "right" | "r" =>
                Ok(Right),
            "up" | "u" =>
                Ok(Up),
            _ =>
                Err(ChryError::InvalidValue(o!(src))),
        }
    }
}


impl Into<u32> for DropItemType {
    fn into(self) -> u32 {
        self as u32
    }
}


impl Default for Position {
    fn default() -> Self {
        Position::TopLeft
    }
}

impl ops::Not for Position {
    type Output = Self;

    fn not(self) -> Self {
        use self::Position::*;

        match self {
            TopLeft => BottomRight,
            TopRight => BottomLeft,
            BottomLeft => TopRight,
            BottomRight => TopLeft,
        }
    }
}

impl FromStr for Position {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Position, ChryError> {
        use self::Position::*;

        match src {
            "top-left" | "left-top" | "tl" =>
                Ok(TopLeft),
            "top-right" | "right-top" | "tr" =>
                Ok(TopRight),
            "bottom-left" | "left-bottom" | "bl" =>
                Ok(BottomLeft),
            "bottom-right" | "right-bottom" | "br" =>
                Ok(BottomRight),
            _ =>
                Err(ChryError::InvalidValue(o!(src))),
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Position::*;

        let name = match *self {
            TopLeft => "top-left",
            TopRight => "top-right",
            BottomLeft => "bottom-left",
            BottomRight => "bottom-right",
        };
        write!(f, "{}", name)
    }
}


impl Default for Views {
    fn default() -> Self {
        Views {
            cols: 1,
            rows: 1,
        }
    }
}

impl Views {
    pub fn len(&self) -> usize {
        self.rows * self.cols
    }
}


fn append_completion_entry(store: &ListStore, entry: &str, historize: bool) {
    if historize {
        let _ = util::file::write_line(entry, &Some(app_path::entry_history()));
    }

    let iter = store.append();
    let value = Value::from(entry);
    store.set_value(&iter, 0, &value);
}

fn load_initial_completion(store: &ListStore) -> Result<(), Box<Error>> {
    let path = app_path::entry_history();
    let lines = util::file::read_lines(&path)?;
    for line in &lines {
        append_completion_entry(store, line, false);
    }
    Ok(())
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

fn reset_scroll(window: &ScrolledWindow, direction: &Direction) {
    use self::Direction::*;

    let f = |horizontal| {
        let adj = if horizontal { window.get_hadjustment() } else { window.get_vadjustment() };
        if_let_some!(adj = adj, ());

        let value = match *direction {
            Right | Down => 0.0,
            Left | Up => adj.get_upper(),
        };

        adj.set_value(value);

        if horizontal {
            window.set_hadjustment(&adj)
        } else {
            window.set_vadjustment(&adj)
        }
    };

    match *direction {
        Left | Right => f(true),
        Up | Down => f(false),
    }
}

fn scroll_window(window: &ScrolledWindow, direction: &Direction, scroll_size_ratio: f64, crush: bool, reset_at_end: bool, count: usize) -> bool {
    use self::Direction::*;

    let scroll = |horizontal| -> bool {
        let adj = if horizontal { window.get_hadjustment() } else { window.get_vadjustment() };
        if_let_some!(adj = adj, false);

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

        if feq(adj.get_value(), value, 0.000_000_1) {
            if reset_at_end {
                reset_scroll(window, direction);
            }
            false
        } else {
            if horizontal { window.set_hadjustment(&adj) } else { window.set_vadjustment(&adj) }
            true
        }
    };

    match *direction {
        Left | Right => scroll(true),
        Up | Down => scroll(false),
    }
}

fn setup_drag<T: WidgetExt + WidgetExtManual >(widget: &T) {
    use gdk::DragAction;
    use gtk::{DestDefaults, TargetEntry, TargetFlags};

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
    widget.drag_dest_set(DestDefaults::ALL, &targets, action);
    widget.add_events(EventMask::SCROLL_MASK.bits() as i32);

}
