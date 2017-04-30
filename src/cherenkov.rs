/* Medama Cherenkov Maker
 * Copyright (C) 2017 anekos <anekos@snca.net>
 *
 * GIMP - The GNU Image Manipulation Program
 * Copyright (C) 1995 Spencer Kimball and Peter Mattis
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

use gdk_pixbuf::Pixbuf;
use rand::distributions::{IndependentSample, Range};
use rand::{self, Rng, ThreadRng};

use color::Color;
use entry::Entry;
use image_buffer::{self, ImageData, ImageBuffer};
use size::{FitTo, Size};
use state::ScalingMethod;
use utils::feq;


type SliceColor = [f64;3];
type TupleColor = (f64, f64, f64);

const FERROR: f64 = 0.000001;


#[derive(Debug, Clone)]
pub struct Che {
    pub center: (f64, f64),
    pub n_spokes: usize,
    pub random_hue: f64,
    pub radius: f64,
    pub color: Color,
}

pub struct Cherenkoved {
    cache: HashMap<Entry, CacheEntry>
}

pub struct CacheEntry {
    image: ImageData,
    cell_size: Size,
    fit_to: FitTo,
    modifiers: Vec<Che>
}


impl Cherenkoved {
    pub fn new() -> Cherenkoved {
        Cherenkoved { cache: HashMap::new() }
    }

    pub fn get_image_data(&mut self, entry: &Entry, cell_size: &Size, fit_to: &FitTo, scaling: &ScalingMethod) -> Result<ImageData, image_buffer::Error> {
        let new_entry = match self.cache.get(entry) {
            None =>
                return image_buffer::get_image_data(entry, cell_size, fit_to, scaling),
            Some(cache_entry) => {
                if cache_entry.is_valid(cell_size, fit_to) {
                    return Ok(cache_entry.image.clone())
                }
                let modifiers = cache_entry.modifiers.clone();
                match self.re_cherenkov(entry, cell_size, fit_to, scaling, &modifiers) {
                    Ok(image) =>
                        CacheEntry {image: image, cell_size: cell_size.clone(), fit_to: fit_to.clone(), modifiers: modifiers},
                    Err(error) =>
                        return Err(error)
                }
            }
        };

        let result = new_entry.image.clone();
        self.cache.insert(entry.clone(), new_entry);
        Ok(result)
    }

    pub fn remove(&mut self, entry: &Entry) {
        self.cache.remove(entry);
    }

    pub fn cherenkov(&mut self, entry: &Entry, cell_size: &Size, fit_to: &FitTo, che: &Che, scaling: &ScalingMethod) {
        if let Some(mut cache_entry) = self.cache.get_mut(entry) {
            if let ImageBuffer::Static(ref mut pixbuf) = cache_entry.image.buffer {
                cache_entry.modifiers.push(che.clone());
                *pixbuf = cherenkov_pixbuf(pixbuf.clone(), che);
            }
            return;
        }

        if let Ok(image) = self.get_image_data(entry, cell_size, fit_to, scaling) {
            if let ImageBuffer::Static(pixbuf) = image.buffer {
                self.cache.insert(
                    entry.clone(),
                    CacheEntry {
                        image: ImageData {
                            buffer: ImageBuffer::Static(cherenkov_pixbuf(pixbuf, che)),
                            size: image.size
                        },
                        cell_size: cell_size.clone(),
                        fit_to: fit_to.clone(),
                        modifiers: vec![],
                    });
            }
        }
    }

    fn re_cherenkov(&self, entry: &Entry, cell_size: &Size, fit_to: &FitTo, scaling: &ScalingMethod, modifiers: &[Che]) -> Result<ImageData, image_buffer::Error> {
        image_buffer::get_image_data(entry, cell_size, fit_to, scaling).map(|mut image| {
            match image.buffer {
                ImageBuffer::Static(mut pixbuf) => {
                    for che in modifiers {
                        pixbuf = cherenkov_pixbuf(pixbuf, che);
                    }
                    image.buffer = ImageBuffer::Static(pixbuf);
                }
                _ => {}
            }
            image
        })
    }
}


impl CacheEntry {
    pub fn is_valid(&self, cell_size: &Size, fit_to: &FitTo) -> bool {
        self.cell_size == *cell_size && self.fit_to == *fit_to
    }
}


fn cherenkov_pixbuf(pixbuf: Pixbuf, che: &Che) -> Pixbuf {
    {
        let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
        let rowstride = pixbuf.get_rowstride();
        let channels = pixbuf.get_n_channels();
        if channels == 4 {
            let pixels: &mut [u8] = unsafe { pixbuf.get_pixels() };
            nova(che, pixels, rowstride, width, height);
        }
    }
    pixbuf
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
fn nova(che: &Che, pixels: &mut [u8], rowstride: i32, width: i32, height: i32) {
    let (cx, cy) = che.center;
    let (cx, cy) = ((width as f64 * cx) as i32, (height as f64 * cy) as i32);
    let radius = clamp(((width * width + height * height) as f64).sqrt() * che.radius, 0.00000001, 100.0);

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
            let u = (x - cx) as f64 / radius;
            let v = (y - cy) as f64 / radius;
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
                let in_color = pixels[ptr + ci] as f64 / 255.0;
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
