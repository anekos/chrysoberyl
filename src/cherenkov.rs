
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str::FromStr;
use std::thread::spawn;

use cairo::{Context, Format, ImageSurface, Pattern, self, SurfacePattern};
use gdk::prelude::ContextExt;
use gdk_pixbuf::{Pixbuf, PixbufExt};

use crate::color::Color;
use crate::entry::image::Imaging;
use crate::entry::{Entry, EntryContent, Key, self};
use crate::errors::{AppResult, AppResultU, AppError};
use crate::gtk_utils::new_pixbuf_from_surface;
use crate::image::{ImageBuffer, StaticImageBuffer};
use crate::size::{Size, Region};
use crate::state::Drawing;

pub mod eye_detector;
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

    pub fn get_image_buffer(&mut self, entry: &Entry, imaging: &Imaging) -> Option<AppResult<ImageBuffer>> {
        if_let_some!(cache_entry = self.cache.get_mut(&entry.key), None);
        Some(get_image_buffer(cache_entry, &entry.content, imaging))
    }

    pub fn generate_animation_gif<T: AsRef<Path>, F>(&self, entry: &Entry, imaging: &Imaging, length: u8, path: &T, on_complete: F) -> AppResultU
    where F: FnOnce() + Send + 'static {
        use gif;
        use gif::SetParameter;
        use crate::image::ImageBuffer::Static;
        use gdk_pixbuf::PixbufExt;
        use std::fs::File;

        fn generate(mut file: File, mut cache_entry: CacheEntry, entry_content: &EntryContent, imaging: &Imaging, size: Size, length: u8) -> AppResultU {
            let (width, height) = (size.width as u16, size.height as u16);

            let mut encoder = gif::Encoder::new(&mut file, width, height, &[])?;
            encoder.set(gif::Repeat::Infinite)?;

            cache_entry.image = None;
            for _ in 0 .. length {
                let mut cache_entry = cache_entry.clone();
                cache_entry.reseed();
                if let Static(buffer) = get_image_buffer(&mut cache_entry, &entry_content, &imaging)? {
                    let pixbuf = buffer.get_pixbuf();
                    let channels = pixbuf.get_n_channels();

                    if channels == 4 {
                        let pixels: &mut [u8] = unsafe { pixbuf.get_pixels() };
                        let frame = gif::Frame::from_rgba(width, height, &mut *pixels);
                        encoder.write_frame(&frame)?;
                    } else {
                        return Err(AppError::Fixed("Invalid channels"));
                    }
                }
            }

            puts_event!("cherenkov/generate_animation_gif/done");
            Ok(())
        }

        if_let_some!(cache_entry = self.cache.get(&entry.key).cloned(), Err(AppError::Fixed("Not cherenkoved")));
        let size = {
            if_let_some!(image = cache_entry.image.as_ref(), Err(AppError::Fixed("Not cherenkoved")));;
            image.get_fit_size()
        };

        let imaging = imaging.clone();
        let entry_content = entry.content.clone();
        let file = File::create(path.as_ref())?;

        spawn(move || {
            if let Err(err) = generate(file, cache_entry, &entry_content, &imaging, size, length) {
                puts_error!(err, "at" => "cherenkoved/generate_animation_gif");
            } else {
                on_complete();
            }
        });

        Ok(())
    }

    pub fn generate_animation_png<T: AsRef<Path>>(&self, entry: &Entry, imaging: &Imaging, length: u8, path: &T) -> AppResultU {
        use apng_encoder::apng;
        use crate::image::ImageBuffer::Static;
        use gdk_pixbuf::PixbufExt;
        use std::fs::File;

        fn generate(mut file: File, mut cache_entry: CacheEntry, entry_content: &EntryContent, imaging: &Imaging, size: Size, length: u8) -> AppResultU {
            let (width, height) = (size.width as u32, size.height as u32);

            let color = apng::Color::RGBA(8);
            let meta = apng::Meta { width, height, color, frames: u32::from(length), plays: None };

            let mut encoder = apng::encoder::Encoder::create(&mut file, meta)?;

            cache_entry.image = None;
            for _ in 0 .. length {
                let mut cache_entry = cache_entry.clone();
                cache_entry.reseed();
                if let Static(buffer) = get_image_buffer(&mut cache_entry, &entry_content, &imaging)? {
                    let pixbuf = buffer.get_pixbuf();
                    let channels = pixbuf.get_n_channels();
                    let row_stride = pixbuf.get_rowstride() as usize;

                    if channels == 4 {
                        let pixels: &mut [u8] = unsafe { pixbuf.get_pixels() };
                        encoder.write_frame(&pixels, None, None, Some(row_stride))?;
                    } else {
                        return Err(AppError::Fixed("Invalid channels"));
                    }
                }
            }

            encoder.finish()?;

            puts_event!("cherenkov/generate_animation_png/done");
            Ok(())
        }

        if_let_some!(cache_entry = self.cache.get(&entry.key).cloned(), Err(AppError::Fixed("Not cherenkoved")));
        let size = {
            if_let_some!(image = cache_entry.image.as_ref(), Err(AppError::Fixed("Not cherenkoved")));;
            image.get_fit_size()
        };

        let imaging = imaging.clone();
        let entry_content = entry.content.clone();
        let file = File::create(path.as_ref())?;

        spawn(move || {
            if let Err(err) = generate(file, cache_entry, &entry_content, &imaging, size, length) {
                puts_error!(err, "at" => "cherenkoved/generate_animation_png");
            }
        });

        Ok(())
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
        if_let_some!(entry = self.cache.get_mut(&entry.key));
        entry.reseed();
    }

    pub fn cherenkov(&mut self, entry: &Entry, imaging: &Imaging, new_modifiers: &[Modifier]) {
        let mut modifiers = self.cache.get(&entry.key).map(|it| it.modifiers.clone()).unwrap_or_else(|| vec![]);

        modifiers.extend_from_slice(new_modifiers);

        if_let_ok!(image_buffer = timeit!("re_cherenkov" => re_cherenkov(&entry.content, imaging, &modifiers)), |_| ());

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

    pub fn reseed(&mut self) {
        for it in &mut self.modifiers {
            if let Che::Nova(ref mut nv) = it.che {
                nv.seed.reset();
            }
        }
        self.expired = true;
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
    type Err = AppError;

    fn from_str(src: &str) -> AppResult<Self> {
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
            _ => return Err(AppError::InvalidValue(o!(src))),
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


fn get_image_buffer(cache_entry: &mut CacheEntry, entry_content: &EntryContent, imaging: &Imaging) -> AppResult<ImageBuffer> {
    if let Some(image) = cache_entry.get(imaging.cell_size, &imaging.drawing) {
        return Ok(ImageBuffer::Static(image))
    }

    let modifiers = cache_entry.modifiers.clone();

    let image = re_cherenkov(entry_content, imaging, &modifiers)?;

    cache_entry.image = Some(image.clone());
    cache_entry.drawing = imaging.drawing.clone();
    cache_entry.cell_size = imaging.cell_size;
    cache_entry.expired = false;
    Ok(ImageBuffer::Static(image))
}

fn re_cherenkov(entry_content: &EntryContent, imaging: &Imaging, modifiers: &[Modifier]) -> AppResult<StaticImageBuffer> {
    let image_buffer = entry::image::get_image_buffer(entry_content, imaging)?;
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
        Err(AppError::Fixed("Not static image"))
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
