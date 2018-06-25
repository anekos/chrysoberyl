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


use std::f64::consts::PI;

use gdk_pixbuf::PixbufExt;
use rand::distributions::{Distribution, Uniform};
use rand::{Rng, RngCore, SeedableRng, self, StdRng};

use color::Color;
use util::num::feq;

use cherenkov::modified::Modified;



type SliceColor = [f64;3];
type TupleColor = (f64, f64, f64);

const FERROR: f64 = 0.000_001;


#[derive(Debug, Clone)]
pub struct Nova {
    pub center: (f64, f64),
    pub color: Color,
    pub n_spokes: usize,
    pub radius: f64,
    pub random_hue: f64,
    pub seed: Seed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Seed {
    fixed: bool,
    value: [u8;32],
}


impl Seed {
    pub fn new(text: &Option<String>) -> Self {
        let mut value = [0;32];
        if let Some(ref text) = text {
            for (i, b) in text.as_bytes().iter().enumerate() {
                value[i % 32] ^= b;
            }
        } else {
            set_seed_randomly(&mut value)
        }
        Seed { fixed: text.is_some(), value }
    }

    pub fn reset(&mut self) {
        set_seed_randomly(&mut self.value);
    }

    pub fn rng(&self) -> StdRng {
        StdRng::from_seed(self.value)
    }
}


#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
pub fn nova_(nv: &Nova, modified: Modified) -> Modified {
    let pixbuf = modified.get_pixbuf();
    let channels = pixbuf.get_n_channels();

    if channels == 4 {
        let (width, height) = (pixbuf.get_width(), pixbuf.get_height());
        let rowstride = pixbuf.get_rowstride();
        let pixels: &mut [u8] = unsafe { pixbuf.get_pixels() };
        nova(nv, pixels, rowstride, width, height);
    }

    Modified::P(pixbuf)
}

#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
pub fn nova(nv: &Nova, pixels: &mut [u8], rowstride: i32, width: i32, height: i32) {
    let (cx, cy) = nv.center;
    let (cx, cy) = ((f64!(width) * cx) as i32, (f64!(height) * cy) as i32);
    let radius = clamp((f64!(width * width + height * height)).sqrt() * nv.radius, 0.000_000_01, 100.0);

    let (spokes, spoke_colors) = {
        let mut rng = nv.seed.rng();
        let mut spokes = vec![];
        let mut spoke_colors: Vec<SliceColor> = vec![];
        let (mut h, s, v) = rgb_to_hsv(nv.color.tupled3());

        for _ in 0 .. nv.n_spokes {
            spokes.push(gauss(&mut rng));
            h += nv.random_hue / 360.0 * range_rand(&mut rng, -0.5, 0.5);

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

            let t = (u.atan2(v) / (2.0 * PI) + 0.51) * nv.n_spokes as f64;
            let i = t.floor() as usize;
            let t = t - i as f64;
            let i = i % nv.n_spokes;

            let w1 = spokes[i] * (1.0 - t) + spokes[(i + 1) % nv.n_spokes] * t;
            let w1 = w1 * w1;

            let w = 1.0 / (l + 0.001) * 0.9;
            let nova_alpha = clamp(w, 0.0, 1.0);
            let compl_ratio = 1.0 - nova_alpha;
            let ptr = (y * rowstride + x * 4 /* RGB+ALPHA */) as usize;

            for ci in 0..3 {
                let in_color = f64!(pixels[ptr + ci]) / 255.0;
                let spoke_color = spoke_colors[i][ci] * (1.0 - t) + spoke_colors[(i + 1) % nv.n_spokes][ci] * t;

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
    let range = Uniform::new(0.0, 1.0);

    for _ in 0..100 {
        assert_color(
            range.sample(&mut rng),
            range.sample(&mut rng),
            range.sample(&mut rng));
    }
}

fn gauss(rng: &mut StdRng) -> f64 {
  let mut sum = 0.0;

  for _ in 0..6 {
      let r: f64 = rng.gen();
      sum += r;
  }

  sum / 6.0
}

fn range_rand (rng: &mut StdRng, from: f64, to: f64) -> f64 {
    Uniform::new(from, to).sample(rng)
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

fn set_seed_randomly(result: &mut [u8;32]) {
    let mut rng = rand::thread_rng();
    for i in 0..32 {
        result[i] = rng.next_u32() as u8;
    }
}
