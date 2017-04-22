

pub struct Size {
    width: i32,
    height: i32
}


impl Size {
    pub new(width: i32, height: i32) -> Size {
        Size { width: width, height: height }
    }

    pub scaled(&self, scale: f64) -> Size {
        Size {
            width: (self.width as f64 * scale) as i32,
            height: (self.height as f64 * scale) as i32,
        }
    }
}
