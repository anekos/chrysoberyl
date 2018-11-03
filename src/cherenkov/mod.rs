
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use cairo::{Context, Format, ImageSurface, Pattern, self, SurfacePattern};
use gdk::prelude::ContextExt;
use gdk_pixbuf::{Pixbuf, PixbufExt};

use color::Color;
use entry::image::Imaging;
use entry::{Entry, Key, self};
use errors::ChryError;
use gtk_utils::new_pixbuf_from_surface;
use image::{ImageBuffer, StaticImageBuffer};
use size::{Size, Region};
use state::Drawing;

pub mod fill;
pub mod modified;
pub mod nova;

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
    Fill(Shape, Region, Color, Option<Operator>, bool),
}

#[derive(Clone)]
pub struct Cherenkoved {
    cache: HashMap<Key, CacheEntry>
}

#[derive(Clone)]
pub struct CacheEntry {
    cell_size: Size,
    drawing: Drawing,
    expired: bool,
    image: Option<StaticImageBuffer>,
    modifiers: Vec<Modifier>,
}

#[derive(Clone, Debug, Copy)]
pub struct Operator(pub cairo::Operator);


impl Cherenkoved {
    pub fn new() -> Cherenkoved {
        Cherenkoved { cache: HashMap::new() }
    }

    pub fn get_image_buffer(&mut self, entry: &Entry, imaging: &Imaging) -> Option<Result<ImageBuffer, Box<Error>>> {
        if_let_some!(cache_entry = self.cache.get_mut(&entry.key), None);
        Some(get_image_buffer(cache_entry, entry, imaging))
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

    pub fn cherenkov1(&mut self, entry: &Entry, imaging: &Imaging, modifier: Modifier) {
        self.cherenkov(entry, imaging, &[modifier])
    }

    pub fn reset(&mut self, entry: &Entry) {
        if_let_some!(entry = self.cache.get_mut(&entry.key), ());
        for it in &mut entry.modifiers {
            if let Che::Nova(ref mut nv) = it.che {
                nv.seed.reset();
            }
        }
        entry.expired = true;
    }

    pub fn cherenkov(&mut self, entry: &Entry, imaging: &Imaging, new_modifiers: &[Modifier]) {
        let mut modifiers = self.cache.get(&entry.key).map(|it| it.modifiers.clone()).unwrap_or_else(|| vec![]);

        modifiers.extend_from_slice(new_modifiers);

        if_let_ok!(image_buffer = time!("re_cherenkov" => re_cherenkov(entry, imaging, &modifiers)), |_| ());

        self.cache.insert(
            entry.key.clone(),
            CacheEntry {
                cell_size: imaging.cell_size,
                drawing: imaging.drawing.clone(),
                expired: false,
                image: Some(image_buffer),
                modifiers,
            });
    }
}


impl CacheEntry {
    pub fn get(&self, cell_size: Size, drawing: &Drawing) -> Option<StaticImageBuffer> {
        if !self.expired && self.cell_size == cell_size && self.drawing.fit_to == drawing.fit_to && self.drawing.clipping == drawing.clipping && self.drawing.mask_operator == drawing.mask_operator {
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


impl Modifier {
    fn fix(&self, original_size: &Option<Size>, drawing: &Drawing) -> Self {
        let che = self.che.fix(original_size, drawing);
        Modifier { che, search_highlight: self.search_highlight }
    }
}


impl Che {
    fn fix(&self, original_size: &Option<Size>, drawing: &Drawing) -> Self {
        if let Che::Nova(ref che) = *self {
            let mut che = che.clone();
            if let Some(clipping) = drawing.clipping {
                let (cw, ch) = (clipping.width(), clipping.height());
                let (x, y) = che.center;
                let x = (x - clipping.left) / cw;
                let y = (y - clipping.top) / ch;
                if let Some(original_size) = *original_size {
                    let r = f64!(original_size.width) / f64!(original_size.height);
                    che.radius *= (r * r + 1.0).sqrt() / (cw * cw * r * r + ch * ch).sqrt();
                }
                che.center = (x, y);
            }
            Che::Nova(che)
        } else {
            self.clone()
        }
    }
}


impl PartialEq for Operator {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Operator {
}

impl Hash for Operator {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use cairo::Operator::*;
        let n = match self.0 {
            Clear         => 1,
            Source        => 2,
            Over          => 3,
            In            => 4,
            Out           => 5,
            Atop          => 6,
            Dest          => 7,
            DestOver      => 8,
            DestIn        => 9,
            DestOut       => 10,
            DestAtop      => 11,
            Xor           => 12,
            Add           => 13,
            Saturate      => 14,
            Multiply      => 15,
            Screen        => 16,
            Overlay       => 17,
            Darken        => 18,
            Lighten       => 19,
            ColorDodge    => 20,
            ColorBurn     => 21,
            HardLight     => 22,
            SoftLight     => 23,
            Difference    => 24,
            Exclusion     => 25,
            HslHue        => 26,
            HslSaturation => 27,
            HslColor      => 28,
            HslLuminosity => 29,
        };
        n.hash(state);
    }
}

impl FromStr for Operator {
    type Err = ChryError;

    fn from_str(src: &str) -> Result<Self, ChryError> {
        use self::cairo::Operator::*;

        let result = match src {
            "clear" => Clear,
            "source" => Source,
            "over" => Over,
            "in" => In,
            "out" => Out,
            "atop" => Atop,
            "dest" => Dest,
            "dest-over" => DestOver,
            "dest-in" => DestIn,
            "dest-out" => DestOut,
            "dest-atop" => DestAtop,
            "xor" => Xor,
            "add" => Add,
            "saturate" => Saturate,
            "multiply" => Multiply,
            "screen" => Screen,
            "overlay" => Overlay,
            "darken" => Darken,
            "lighten" => Lighten,
            "color-dodge" => ColorDodge,
            "color-burn" => ColorBurn,
            "hard-light" => HardLight,
            "soft-light" => SoftLight,
            "difference" => Difference,
            "exclusion" => Exclusion,
            "hsl-hue" => HslHue,
            "hsl-saturation" => HslSaturation,
            "hsl-color" => HslColor,
            "hsl-luminosity" => HslLuminosity,
            _ => return Err(ChryError::InvalidValue(o!(src))),
        };

        Ok(Operator(result))
    }
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use cairo::Operator::*;

        let result =
            match self.0 {
                Clear => "clear",
                Source => "source",
                Over => "over",
                In => "in",
                Out => "out",
                Atop => "atop",
                Dest => "dest",
                DestOver => "dest-over",
                DestIn => "dest-in",
                DestOut => "dest-out",
                DestAtop => "dest-atop",
                Xor => "xor",
                Add => "add",
                Saturate => "saturate",
                Multiply => "multiply",
                Screen => "screen",
                Overlay => "overlay",
                Darken => "darken",
                Lighten => "lighten",
                ColorDodge => "color-dodge",
                ColorBurn => "color-burn",
                HardLight => "hard-light",
                SoftLight => "soft-light",
                Difference => "difference",
                Exclusion => "exclusion",
                HslHue => "hsl-hue",
                HslSaturation => "hsl-saturation",
                HslColor => "hsl-color",
                HslLuminosity => "hsl-luminosity",
            };

        write!(f, "{}", result)
    }
}


fn get_image_buffer(cache_entry: &mut CacheEntry, entry: &Entry, imaging: &Imaging) -> Result<ImageBuffer, Box<Error>> {
    if let Some(image) = cache_entry.get(imaging.cell_size, &imaging.drawing) {
        return Ok(ImageBuffer::Static(image))
    }

    let modifiers = cache_entry.modifiers.clone();

    let image = re_cherenkov(entry, imaging, &modifiers)?;

    cache_entry.image = Some(image.clone());
    cache_entry.drawing = imaging.drawing.clone();
    cache_entry.cell_size = imaging.cell_size;
    cache_entry.expired = false;
    Ok(ImageBuffer::Static(image))
}

fn re_cherenkov(entry: &Entry, imaging: &Imaging, modifiers: &[Modifier]) -> Result<StaticImageBuffer, Box<Error>> {
    let image_buffer = entry::image::get_image_buffer(entry, imaging)?;
    if let ImageBuffer::Static(buf) = image_buffer {
        let mut mask = None;
        let mut modified = Modified::P(buf.get_pixbuf());
        for modifier in modifiers {
            let modifier = modifier.fix(&buf.original_size, &imaging.drawing);
            let (_modified, _mask) = cherenkov_pixbuf(modified, mask, &modifier.che);
            modified = _modified;
            mask = _mask;
        }
        let pixbuf = modified.get_pixbuf();
        let pixbuf = if let Some(mask) = mask {
            apply_mask(&pixbuf, &mask, imaging.drawing.mask_operator.0)
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
        Che::Fill(shape, ref region, color, operator, false) =>
            (fill::fill(shape, region, color, operator, modified), mask_surface),
        Che::Fill(shape, ref region, color, operator, true) => {
            let mask_surface =  fill::mask(mask_surface, shape, region, color, operator, &modified);
            (modified, Some(mask_surface))
        }
    }
}

fn apply_mask(pixbuf: &Pixbuf, mask: &ImageSurface, operator: cairo::Operator) -> Pixbuf {
    let (w, h) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, w, h).unwrap();
    let context = Context::new(&surface);

    context.set_source_pixbuf(pixbuf, 0.0, 0.0);
    context.paint();

    context.set_operator(operator);
    let pattern = Pattern::SurfacePattern(SurfacePattern::create(&mask));
    context.mask(&pattern);

    new_pixbuf_from_surface(&surface)
}
