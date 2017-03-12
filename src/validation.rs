
use std::path::Path;


pub fn is_valid_image_filename<T: AsRef<Path>>(path: T) -> bool {
    if let Some(extension) = path.as_ref().to_path_buf().extension() {
        match &*extension.to_str().unwrap().to_lowercase() {
            "jpeg" | "jpg" | "png" | "gif" => true,
            _ => false
        }
    } else {
        false
    }
}
