
use std::f64::consts::PI;

use cairo::{Context, ImageSurface, Format};

use crate::color::Color;
use crate::size::Region;

use crate::cherenkov::Operator;
use crate::cherenkov::modified::Modified;



#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Rectangle,
    Circle,
    Ellipse,
}

pub struct Parameter<'a> {
    pub che: &'a Region,
    pub clipping: &'a Option<Region>,
    pub color: Color,
    pub operator: Option<Operator>,
    pub shape: Shape,
}

struct ContextParamater<'a> {
    clipping: &'a Option<Region>,
    color: Color,
    context: &'a Context,
    h: i32,
    operator: Option<Operator>,
    region: &'a Region,
    shape: Shape,
    w: i32,
}


#[cfg_attr(feature = "cargo-clippy", allow(clippy::many_single_char_names))]
pub fn fill(modified: Modified, p: Parameter) -> Modified {
    let surface = modified.get_image_surface();
    let context = Context::new(&surface);

    context_fill(ContextParamater {
        clipping: &p.clipping, 
        color: p.color,
        context: &context,
        h: surface.get_height(),
        operator: p.operator,
        region: &p.che,
        shape: p.shape,
        w: surface.get_width(),
    });

    Modified::S(surface)
}

pub fn mask(surface: Option<ImageSurface>, modified: &Modified, p: Parameter) -> ImageSurface {
    let size = modified.get_size();
    let surface = surface.unwrap_or_else(|| ImageSurface::create(Format::ARgb32, size.width, size.height).unwrap());
    let context = Context::new(&surface);

    context_fill(ContextParamater {
        clipping: &p.clipping,
        color: p.color,
        context: &context,
        h: size.height,
        operator: p.operator,
        region: &p.che,
        shape: p.shape,
        w: size.width,
    });

    surface
}

#[allow(clippy::many_single_char_names)]
fn context_fill<'a>(p: ContextParamater) {
    let (r, g, b, a) = p.color.tupled4();
    p.context.set_source_rgba(r, g, b, a);

    p.context.save();

    let region = p.clipping.map(|it| p.region.clipped(&it)).unwrap_or(*p.region);

    match p.shape {
        Shape::Rectangle => {
            let (w, h) = (f64!(p.w), f64!(p.h));
            p.context.rectangle(
                region.left * w,
                region.top * h,
                (region.right - region.left) * w,
                (region.bottom - region.top) * h);
        },
        Shape::Circle => {
            let (w, h) = (f64!(p.w), f64!(p.h));
            let (rw, rh) = (region.width(), region.height());
            let r = min!(rw * w, rh * h) / 2.0;
            p.context.arc(
                (region.left + rw / 2.0) * w,
                (region.top + rh / 2.0) * h,
                r,
                0.0,
                2.0 * PI);
        },
        Shape::Ellipse => {
            let (w, h) = (f64!(p.w), f64!(p.h));
            let (rw, rh) = (region.width(), region.height());
            p.context.translate(
                (region.left + rw / 2.0) * w,
                (region.top + rh / 2.0) * h);
            p.context.scale(rw * w / 2.0, rh * h / 2.0);
            p.context.arc(0.0, 0.0, 1.0, 0.0, 2.0 * PI);
        }
    }
    if let Some(operator) = p.operator {
        p.context.set_operator(operator.0);
    }
    p.context.fill();

    p.context.restore();
}
