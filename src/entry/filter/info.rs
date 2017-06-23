
use std::path::Path;

use immeta;

use archive::ArchiveEntry;
use entry::{Entry, EntryInfo, EntryContent};
use size::Size;



pub fn get_info(entry: &mut Entry) -> &EntryInfo {
    let info = &mut entry.info;
    let path = o!(entry.key.1);
    let content = &entry.content;

    info.get(|| generate_info(content, path))
}

fn generate_info(content: &EntryContent, path: String) -> EntryInfo {
    use self::EntryContent::*;

    let size = match *content {
        File(ref path) | Http(ref path, _) =>
            generate_static_image_size(path),
        Archive(_, ref entry) =>
            generate_archive_image_size(entry),
        Pdf(_, _) =>
            None,
    };

    EntryInfo {
        size: size,
        path: path,
    }
}

fn generate_static_image_size(path: &Path) -> Option<Size> {
    let img = immeta::load_from_file(path).ok();
    img.map(|img| {
        let dim = img.dimensions();
        Size::new(dim.width as i32, dim.height as i32)
    })
}

fn generate_archive_image_size(entry: &ArchiveEntry) -> Option<Size> {
    let buf = &*entry.content;
    let img = immeta::load_from_buf(buf.as_slice()).ok();
    img.map(|img| {
        let dim = img.dimensions();
        Size::new(dim.width as i32, dim.height as i32)
    })
}
