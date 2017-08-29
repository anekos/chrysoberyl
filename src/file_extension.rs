
use std::path::Path;

use entry::EntryType;



pub fn get_entry_type_from_filename<T: AsRef<Path>>(path: &T) -> Option<EntryType> {
    if_let_some!(ext = path.as_ref().extension(), None);
    if_let_some!(ext = ext.to_str(), None);
    let ext = ext.to_lowercase();

    match &*ext {
        "zip" | "rar" | "tar.gz" | "lzh" | "lha" =>
            Some(EntryType::Archive),
        "pdf" =>
            Some(EntryType::PDF),
        "ani" | "bmp" | "cur" | "gif" | "icns" | "ico" | "j2k" | "jp2" | "jpc" | "jpe" | "jpeg" | "jpf" | "jpg" | "jpx" | "pbm" | "pgm" | "png" | "pnm" | "ppm" | "qif" | "qtif" | "svg" | "svg.gz" | "svgz" | "targa" | "tga" | "tif" | "tiff" | "xbm" | "xpm" =>
            Some(EntryType::Image),
        _ =>
            None
    }
}

pub fn is_valid_image_filename<T: AsRef<Path>>(path: &T) -> bool {
    get_entry_type_from_filename(path) == Some(EntryType::Image)
}
