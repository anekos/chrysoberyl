
use cairo::{ImageSurface, Context, Format};
use gdk::prelude::ContextExt;
use gdk_pixbuf::{Pixbuf, PixbufExt};

use gtk_utils::new_pixbuf_from_surface;
use size::Size;



pub enum Modified {
    P(Pixbuf),
    S(ImageSurface),
}


impl Modified {
    pub fn get_pixbuf(self) -> Pixbuf {
        use self::Modified::*;

        match self {
            P(pixbuf) => pixbuf,
            S(surface) => new_pixbuf_from_surface(&surface)
        }
    }

    pub fn get_image_surface(self) -> ImageSurface {
        use self::Modified::*;

        match self {
            P(pixbuf) => {
                let surface = ImageSurface::create(Format::ARgb32, pixbuf.get_width(), pixbuf.get_height()).unwrap();
                let context = Context::new(&surface);
                context.set_source_pixbuf(&pixbuf, 0.0, 0.0);
                context.paint();
                surface

            }
            S(surface) => surface,
        }
    }

    pub fn get_size(&self) -> Size {
        use self::Modified::*;

        match *self {
            P(ref pixbuf) => Size::new(pixbuf.get_width(), pixbuf.get_height()),
            S(ref surface) => Size::new(surface.get_width(), surface.get_height()),
        }
    }
}
