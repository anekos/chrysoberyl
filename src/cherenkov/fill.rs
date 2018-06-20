
use std::f64::consts::PI;

use cairo::{Context, ImageSurface, Format};

use color::Color;
use size::Region;

use cherenkov::Operator;
use cherenkov::modified::Modified;



#[derive(Debug, Clone, Copy)]
pub enum Shape {
    Rectangle,
    Circle,
    Ellipse,
}



#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
pub fn fill(shape: Shape, che: &Region, color: &Color, operator: Option<Operator>, modified: Modified) -> Modified {
    let surface = modified.get_image_surface();
    let context = Context::new(&surface);

    context_fill(&context, shape, che, color, operator, surface.get_width(), surface.get_height());

    Modified::S(surface)
}

pub fn mask(surface: Option<ImageSurface>, shape: Shape, che: &Region, color: &Color, operator: Option<Operator>, modified: &Modified) -> ImageSurface {
    let size = modified.get_size();
    let surface = surface.unwrap_or_else(|| ImageSurface::create(Format::ARgb32, size.width, size.height).unwrap());
    let context = Context::new(&surface);

    context_fill(&context, shape, che, color, operator, size.width, size.height);

    surface
}

#[cfg_attr(feature = "cargo-clippy", allow(many_single_char_names))]
fn context_fill(context: &Context, shape: Shape, region: &Region, color: &Color, operator: Option<Operator>, w: i32, h: i32) {
    let (r, g, b, a) = color.tupled4();
    context.set_source_rgba(r, g, b, a);

    context.save();

    match shape {
        Shape::Rectangle => {
            let (w, h) = (f64!(w), f64!(h));
            context.rectangle(
                region.left * w,
                region.top * h,
                (region.right - region.left) * w,
                (region.bottom - region.top) * h);
        },
        Shape::Circle => {
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
        Shape::Ellipse => {
            let (w, h) = (f64!(w), f64!(h));
            let (rw, rh) = (region.width(), region.height());
            context.translate(
                (region.left + rw / 2.0) * w,
                (region.top + rh / 2.0) * h);
            context.scale(rw * w / 2.0, rh * h / 2.0);
            context.arc(0.0, 0.0, 1.0, 0.0, 2.0 * PI);
        }
    }
    if let Some(operator) = operator {
        context.set_operator(operator.0);
    }
    context.fill();

    context.restore();
}
