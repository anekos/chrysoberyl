
use std::path::Path;

use crate::entry::EntryType;



pub fn get_entry_type_from_filename<T: AsRef<Path>>(path: &T) -> Option<EntryType> {
    let ext = path.as_ref().extension()?;
    let ext = ext.to_str()?;
    let ext = ext.to_lowercase();

    match &*ext {
        // https://github.com/libarchive/libarchive/wiki/LibarchiveFormats
        "7z" | "ar" | "cab" | "cbz" | "cpio" | "iso9660" | "lha" | "lzh" | "mtree" | "pax" | "rar" | "shar" | "tar" | "xar" | "zip" =>
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
