
use std::path::Path;

use immeta;

use archive::ArchiveEntry;
use entry::{Entry, EntryInfo, EntryContent};
use size::Size;



pub fn get_info(entry: &mut Entry) -> &EntryInfo {
    let info = &mut entry.info;
    let content = &entry.content;

    info.get(|| generate_info(content))
}

fn generate_info(content: &EntryContent) -> EntryInfo {
    use self::EntryContent::*;

    match *content {
        File(ref path) | Http(ref path, _) =>
            generate_static_image_info(path),
        Archive(_, ref entry) =>
            generate_archive_image_info(entry),
        Pdf(_, _) =>
            EntryInfo { size: None }
    }
}

fn generate_static_image_info(path: &Path) -> EntryInfo {
    let img = immeta::load_from_file(path).ok();
    EntryInfo {
        size: img.map(|img| {
            let dim = img.dimensions();
            Size::new(dim.width as i32, dim.height as i32)
        })
    }
}

fn generate_archive_image_info(entry: &ArchiveEntry) -> EntryInfo {
    let buf = &*entry.content;
    let img = immeta::load_from_buf(buf.as_slice()).ok();
    EntryInfo {
        size: img.map(|img| {
            let dim = img.dimensions();
            Size::new(dim.width as i32, dim.height as i32)
        })
    }
}
