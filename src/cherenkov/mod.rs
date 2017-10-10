
use std::collections::HashMap;
use std::error::Error;

use cairo::{Context, ImageSurface, Format, SurfacePattern, Operator};
use gdk::prelude::ContextExt;
use gdk_pixbuf::Pixbuf;

use color::Color;
use entry::{Entry, Key, self};
use errors::ChryError;
use gtk_utils::new_pixbuf_from_surface;
use image::{ImageBuffer, StaticImageBuffer};
use size::{Size, Region};
use state::DrawingState;

pub mod fill;
pub mod nova;
pub mod modified;

use self::fill::Shape;
use self::modified::Modified;



#[derive(Debug, Clone)]
pub struct Modifier {
    pub che: Che,
    pub search_highlight: bool,
}

#[derive(Debug, Clone)]
pub enum Che {
    Nova(nova::Nova),
    Fill(Shape, Region, Color, bool),
}

#[derive(Clone)]
pub struct Cherenkoved {
    cache: HashMap<Key, CacheEntry>
}

#[derive(Clone)]
pub struct CacheEntry {
    image: Option<StaticImageBuffer>,
    cell_size: Size,
    drawing: DrawingState,
    modifiers: Vec<Modifier>,
    expired: bool,
}



impl Cherenkoved {
    pub fn new() -> Cherenkoved {
        Cherenkoved { cache: HashMap::new() }
    }

    pub fn get_image_buffer(&mut self, entry: &Entry, cell_size: &Size, drawing: &DrawingState) -> Option<Result<ImageBuffer, Box<Error>>> {
        if_let_some!(cache_entry = self.cache.get_mut(&entry.key), None);
        Some(get_image_buffer(cache_entry, entry, cell_size, drawing))
    }

    pub fn remove(&mut self, key: &Key) {
        self.cache.remove(key);
    }

    pub fn clear_search_highlights(&mut self) -> bool {
        for it in self.cache.values_mut() {
            it.clear_search_highlights();
        }
        let before = self.cache.len();
        self.cache.retain(|_, v| !v.modifiers.is_empty());
        before != self.cache.len()
    }

    pub fn clear_entry_search_highlights(&mut self, entry: &Entry) -> bool {
        if_let_some!(entry = self.cache.get_mut(&entry.key), false);
        entry.clear_search_highlights()
    }

    pub fn undo(&mut self, key: &Key, count: usize) {
        if let Some(cache_entry) = self.cache.get_mut(key) {
            for _ in 0..count {
                cache_entry.modifiers.pop();
            }
            cache_entry.image = None;
        }
    }

    pub fn cherenkov1(&mut self, entry: &Entry, cell_size: &Size, modifier: Modifier, drawing: &DrawingState) {
        self.cherenkov(entry, cell_size, &[modifier], drawing)
    }

    pub fn cherenkov(&mut self, entry: &Entry, cell_size: &Size, new_modifiers: &[Modifier], drawing: &DrawingState) {
        let mut modifiers = self.cache.get(&entry.key).map(|it| it.modifiers.clone()).unwrap_or_else(|| vec![]);

        modifiers.extend_from_slice(new_modifiers);

        if_let_ok!(image_buffer = time!("re_cherenkov" => re_cherenkov(entry, cell_size, drawing, &modifiers)), |_| ());

        self.cache.insert(
            entry.key.clone(),
            CacheEntry {
                image: Some(image_buffer),
                cell_size: *cell_size,
                drawing: drawing.clone(),
                modifiers: modifiers,
                expired: false,
            });
    }
}


impl CacheEntry {
    pub fn get(&self, cell_size: &Size, drawing: &DrawingState) -> Option<StaticImageBuffer> {
        if !self.expired && self.cell_size == *cell_size && self.drawing.fit_to == drawing.fit_to && self.drawing.clipping == drawing.clipping && self.drawing.mask_operator == drawing.mask_operator {
            if let Some(ref image) = self.image {
                return Some(image.clone());
            }
        }
        None
    }

    pub fn clear_search_highlights(&mut self) -> bool {
        let before = self.modifiers.len();
        self.modifiers.retain(|it| !it.search_highlight);
        let changed = before != self.modifiers.len();
        self.expired = changed;
        changed
    }
}


fn get_image_buffer(cache_entry: &mut CacheEntry, entry: &Entry, cell_size: &Size, drawing: &DrawingState) -> Result<ImageBuffer, Box<Error>> {
    if let Some(image) = cache_entry.get(cell_size, drawing) {
        return Ok(ImageBuffer::Static(image))
    }

    let modifiers = cache_entry.modifiers.clone();

    let image = re_cherenkov(entry, cell_size, drawing, &modifiers)?;

    cache_entry.image = Some(image.clone());
    cache_entry.drawing = drawing.clone();
    cache_entry.cell_size = *cell_size;
    Ok(ImageBuffer::Static(image))
}

fn re_cherenkov(entry: &Entry, cell_size: &Size, drawing: &DrawingState, modifiers: &[Modifier]) -> Result<StaticImageBuffer, Box<Error>> {
    let image_buffer = entry::image::get_image_buffer(entry, cell_size, drawing)?;
    if let ImageBuffer::Static(buf) = image_buffer {
        let mut mask = None;
        let mut modified = Modified::P(buf.get_pixbuf());
        for modifier in modifiers {
            let (_modified, _mask) = cherenkov_pixbuf(modified, mask, &modifier.che);
            modified = _modified;
            mask = _mask;
        }
        let pixbuf = modified.get_pixbuf();
        let pixbuf = if let Some(mask) = mask {
            apply_mask(&pixbuf, mask, drawing.mask_operator.0)
        } else {
            pixbuf
        };
        Ok(StaticImageBuffer::new_from_pixbuf(&pixbuf, buf.original_size))
    } else {
        Err(Box::new(ChryError::Fixed("Not static image")))
    }
}

fn cherenkov_pixbuf(modified: Modified, mask_surface: Option<ImageSurface>, che: &Che) -> (Modified, Option<ImageSurface>) {
    match *che {
        Che::Nova(ref che) => (nova::nova_(che, modified), mask_surface),
        Che::Fill(shape, ref region, ref color, false) =>
            (fill::fill(shape, region, color, modified), mask_surface),
        Che::Fill(shape, ref region, ref color, true) => {
            let mask_surface =  fill::mask(mask_surface, shape, region, color, &modified);
            (modified, Some(mask_surface))
        }
    }
}

fn apply_mask(pixbuf: &Pixbuf, mask: ImageSurface, operator: Operator) -> Pixbuf {
    let (w, h) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
    let context = Context::new(&surface);

    context.set_source_pixbuf(pixbuf, 0.0, 0.0);
    context.paint();

    context.set_operator(operator);
    let pattern = SurfacePattern::create(&mask);
    context.mask(&pattern);

    new_pixbuf_from_surface(&surface)
}
