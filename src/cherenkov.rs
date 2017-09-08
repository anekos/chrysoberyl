/* Medama Cherenkov Maker
 * Copyright (C) 2017 anekos <anekos@snca.net>
 *
 * Supernova plug-in
 * Copyright (C) 1997 Eiichi Takamori <taka@ma1.seikyou.ne.jp>,
 *                    Spencer Kimball, Federico Mena Quintero
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place - Suite 330, Boston, MA 02111-1307, USA.
 */


use std::collections::HashMap;
use std::f64::consts::PI;

use cairo::{Context, ImageSurface, Format, SurfacePattern, Operator};
use gdk::prelude::ContextExt;
use gdk_pixbuf::Pixbuf;
use rand::distributions::{IndependentSample, Range};
use rand::{self, Rng, ThreadRng};

use color::Color;
use entry::{Entry, Key, self};
use gtk_utils::new_pixbuf_from_surface;
use image::{ImageBuffer, StaticImageBuffer};
use size::{Size, Region};
use state::DrawingState;
use utils::feq;


type SliceColor = [f64;3];
type TupleColor = (f64, f64, f64);

const FERROR: f64 = 0.000001;


#[derive(Debug, Clone)]
pub struct Modifier {
    pub che: Che,
    pub search_highlight: bool,
}

#[derive(Debug, Clone)]
pub enum Che {
    Nova(CheNova),
    Fill(Filler, Region, Color, bool),
}

#[derive(Debug, Clone, Copy)]
pub enum Filler {
    Rectangle,
    Circle,
    Ellipse,
}

#[derive(Debug, Clone)]
pub struct CheNova {
    pub center: (f64, f64),
    pub n_spokes: usize,
    pub random_hue: f64,
    pub radius: f64,
    pub color: Color,
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

    pub fn get_image_buffer(&mut self, entry: &Entry, cell_size: &Size, drawing: &DrawingState) -> Option<Result<ImageBuffer, String>> {
        if_let_some!(cache_entry = self.cache.get_mut(&entry.key), None);

        if let Some(image) = cache_entry.get(cell_size, drawing) {
            return Some(Ok(ImageBuffer::Static(image)))
        }

        let modifiers = cache_entry.modifiers.clone();

        if_let_ok!(image = re_cherenkov(entry, cell_size, drawing, &modifiers), |err| Some(Err(err)));

        cache_entry.image = Some(image.clone());
        cache_entry.drawing = drawing.clone();
        cache_entry.cell_size = *cell_size;
        Some(Ok(ImageBuffer::Static(image)))
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

    pub fn cherenkov(&mut self, entry: &Entry, cell_size: &Size, modifier: Modifier, drawing: &DrawingState) {
        let mut modifiers = self.cache.get(&entry.key).map(|it| it.modifiers.clone()).unwrap_or_else(|| vec![]);

        modifiers.push(modifier);

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


fn re_cherenkov(entry: &Entry, cell_size: &Size, drawing: &DrawingState, modifiers: &[Modifier]) -> Result<StaticImageBuffer, String> {
    entry::image::get_image_buffer(entry, cell_size, drawing).and_then(|image_buffer| {
        if let ImageBuffer::Static(buf) = image_buffer {
            let mut pixbuf = buf.get_pixbuf();
            let mut mask = None;
            for modifier in modifiers {
                let (_pixbuf, _mask) = cherenkov_pixbuf(pixbuf, mask, &modifier.che);
                pixbuf = _pixbuf;
                mask = _mask;
            }
            let pixbuf = if let Some(mask) = mask {
                apply_mask(&pixbuf, mask, drawing.mask_operator.0)
            } else {
                pixbuf
            };
            Ok(StaticImageBuffer::new_from_pixbuf(&pixbuf, buf.original_size))
        } else {
            Err(o!("Not static image"))
        }
    })
}

fn cherenkov_pixbuf(pixbuf: Pixbuf, mask_surface: Option<ImageSurface>, che: &Che) -> (Pixbuf, Option<ImageSurface>) {
    match *che {
        Che::Nova(ref che) => {
            let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
            let rowstride = pixbuf.get_rowstride();
            let channels = pixbuf.get_n_channels();
            if channels == 4 {
                let pixels: &mut [u8] = unsafe { pixbuf.get_pixels() };
                nova(che, pixels, rowstride, width, height);
            }
            (pixbuf, mask_surface)
        },
        Che::Fill(filler, ref region, ref color, false) =>
            (fill(filler, region, color, &pixbuf), mask_surface),
        Che::Fill(filler, ref region, ref color, true) => {
            let (pixbuf, mask_surface) = mask(mask_surface, filler, region, color, pixbuf);
            (pixbuf, Some(mask_surface))
        }
    }
}

fn gauss(rng: &mut ThreadRng) -> f64 {
  let mut sum = 0.0;

  for _ in 0..6 {
    sum += rng.next_f64();
  }

  sum / 6.0
}

fn range_rand (rng: &mut ThreadRng, from: f64, to: f64) -> f64 {
    Range::new(from, to).ind_sample(rng)
}

#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
fn rgb_to_hsv(rgb: TupleColor) -> TupleColor {
    let (r, g, b) = rgb;
    let max = max!(r, g, b);
    let min = min!(r, g, b);

    let mut h = max - min;

    if h > 0.0 {
        if feq(max, r, FERROR) {
            h = (g - b) / h;
            if h < 0.0 {
                h += 6.0
            }
        } else if feq(max, g, FERROR) {
            h = 2.0 + (b - r) / h
        } else {
            h = 4.0 + (r - g) / h
        }
    }
    h /= 6.0;

    let mut s = max - min;
    if max != 0.0 {
        s /= max;
    }

    (h, s, max)
}

static HRTBL: &'static [&'static [usize;3];6] = &[
    &[0, 3, 1],
    &[2, 0, 1],
    &[1, 0, 3],
    &[1, 2, 0],
    &[3, 1, 0],
    &[0, 1, 2]
];

#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
fn hsv_to_rgb(hsv: TupleColor) -> SliceColor {
    let (h, s, v) = hsv;

    if s == 0.0 {
        return [v, v, v];
    }

    let h = h * 6.0;
    let i = h.floor();

    let f = h - i;
    let rs = &[v, v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f))];
    let idx = HRTBL[i as usize];

    [rs[idx[0]], rs[idx[1]], rs[idx[2]]]
}

fn clamp<T: PartialOrd>(v: T, from: T, to: T) -> T {
  if v < from {
      from
  } else if v > to {
      to
  } else {
      v
  }
}

#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
fn nova(che: &CheNova, pixels: &mut [u8], rowstride: i32, width: i32, height: i32) {
    let (cx, cy) = che.center;
    let (cx, cy) = ((f64!(width) * cx) as i32, (f64!(height) * cy) as i32);
    let radius = clamp((f64!(width * width + height * height)).sqrt() * che.radius, 0.00000001, 100.0);

    let (spokes, spoke_colors) = {
        let mut rng = rand::thread_rng();
        let mut spokes = vec![];
        let mut spoke_colors: Vec<SliceColor> = vec![];
        let (mut h, s, v) = rgb_to_hsv(che.color.tupled3());

        for _ in 0 .. che.n_spokes {
            spokes.push(gauss(&mut rng));
            h += che.random_hue / 360.0 * range_rand(&mut rng, -0.5, 0.5);

            if h < 0.0 {
                h += 1.0;
            } else if h >= 1.0 {
                h -= 1.0;
            }

            let rgb = hsv_to_rgb((h, s, v));
            spoke_colors.push(rgb);
        }

        (spokes, spoke_colors)
    };

    for y in 0..height {
        for x in 0..width {
            let u = f64!(x - cx) / radius;
            let v = f64!(y - cy) / radius;
            let l = (u * u + v * v).sqrt();

            let t = (u.atan2(v) / (2.0 * PI) + 0.51) * che.n_spokes as f64;
            let i = t.floor() as usize;
            let t = t - i as f64;
            let i = i % che.n_spokes;

            let w1 = spokes[i] * (1.0 - t) + spokes[(i + 1) % che.n_spokes] * t;
            let w1 = w1 * w1;

            let w = 1.0 / (l + 0.001) * 0.9;
            let nova_alpha = clamp(w, 0.0, 1.0);
            let compl_ratio = 1.0 - nova_alpha;
            let ptr = (y * rowstride + x * 4 /* RGB+ALPHA */) as usize;

            for ci in 0..3 {
                let in_color = f64!(pixels[ptr + ci]) / 255.0;
                let spoke_color = spoke_colors[i][ci] * (1.0 - t) + spoke_colors[(i + 1) % che.n_spokes][ci] * t;

                let mut out_color = if w > 1.0 {
                    clamp(spoke_color * w, 0.0, 1.0)
                } else {
                    in_color * compl_ratio + spoke_color * nova_alpha
                };

                let c = clamp(w1 * w, 0.0, 1.0);
                out_color += c;
                out_color *= 255.0;
                pixels[ptr + ci] = clamp(out_color, 0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
fn fill(filler: Filler, che: &Region, color: &Color, pixbuf: &Pixbuf) -> Pixbuf {
    let (w, h) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, w, h);
    let context = Context::new(&surface);

    context.set_source_pixbuf(pixbuf, 0.0, 0.0);
    context.paint();

    context_fill(&context, filler, che, color, w, h);

    new_pixbuf_from_surface(&surface)
}

fn mask(surface: Option<ImageSurface>, filler: Filler, che: &Region, color: &Color, pixbuf: Pixbuf) -> (Pixbuf, ImageSurface) {
    let (w, h) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = surface.unwrap_or_else(|| ImageSurface::create(Format::ARgb32, w, h));
    let context = Context::new(&surface);

    context_fill(&context, filler, che, color, w, h);

    (pixbuf, surface)
}

#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
fn context_fill(context: &Context, filler: Filler, region: &Region, color: &Color, w: i32, h: i32) {
    let (r, g, b, a) = color.tupled4();
    context.set_source_rgba(r, g, b, a);

    context.save();

    match filler {
        Filler::Rectangle => {
            let (w, h) = (f64!(w), f64!(h));
            context.rectangle(
                region.left * w,
                region.top * h,
                (region.right - region.left) * w,
                (region.bottom - region.top) * h);
        },
        Filler::Circle => {
            let (w, h) = (f64!(w), f64!(h));
            let (rw, rh) = (region.width(), region.height());
            let r = min!(rw * w, rh * h) / 2.0;
            context.arc(
                (region.left + rw / 2.0) * w,
                (region.top + rh / 2.0) * h,
                r,
                0.0,
                2.0 * PI);
        },
        Filler::Ellipse => {
            let (w, h) = (f64!(w), f64!(h));
            let (rw, rh) = (region.width(), region.height());
            context.translate(
                (region.left + rw / 2.0) * w,
                (region.top + rh / 2.0) * h);
            context.scale(rw * w / 2.0, rh * h / 2.0);
            context.arc(0.0, 0.0, 1.0, 0.0, 2.0 * PI);
        }
    }
    context.fill();

    context.restore();
}

fn apply_mask(pixbuf: &Pixbuf, mask: ImageSurface, operator: Operator) -> Pixbuf {
    let (w, h) = (pixbuf.get_width(), pixbuf.get_height());
    let surface = ImageSurface::create(Format::ARgb32, w, h);
    let context = Context::new(&surface);

    context.set_source_pixbuf(pixbuf, 0.0, 0.0);
    context.paint();

    context.set_operator(operator);
    let pattern = SurfacePattern::create(&mask);
    context.mask(&pattern);

    new_pixbuf_from_surface(&surface)
}

#[cfg(test)]#[test]
fn test_color_converter() {
    fn near(x: f64, y: f64) -> bool {
        (x - y).abs() < 0.001
    }

    fn assert_color(r: f64, g: f64, b: f64) {
        let hsv = rgb_to_hsv((r, g, b));
        let rgb = hsv_to_rgb(hsv);
        assert!(
            near(rgb[0], r) && near(rgb[1], g) && near(rgb[2], b),
            format!("RGB({:?}) => HSV({:?}) => RGB({:?})", (r, g, b), hsv, (rgb[0], rgb[1], rgb[2])));
    }

    assert_color(0.2, 0.2, 0.2);
    assert_color(0.1, 0.2, 0.4);
    assert_color(0.4, 0.2, 0.3);

    let mut rng = rand::thread_rng();
    let range = Range::new(0.0, 1.0);

    for _ in 0..100 {
        assert_color(
            range.ind_sample(&mut rng),
            range.ind_sample(&mut rng),
            range.ind_sample(&mut rng));
    }
}
