
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

    let meta = match *content {
        File(ref path) | Http(ref path, _) =>
            generate_static_image_size(path),
        Archive(_, ref entry) =>
            generate_archive_image_size(entry),
        Pdf(_, _) =>
            None,
    };

    let extension = Path::new(&path).extension().and_then(|it| it.to_str().map(|it| it.to_owned()));

    let entry_type: &'static str = match *content {
        File(_) => "file",
        Http(_, _) => "http",
        Archive(_, _) => "archive",
        Pdf(_, _) => "pdf",
    };

    EntryInfo {
        dimensions: meta.map(|it| it.0),
        path: path,
        extension: extension,
        entry_type: entry_type,
        is_animated: meta.map(|it| it.1).unwrap_or(false),
    }
}

fn generate_static_image_size(path: &Path) -> Option<(Size, bool)> {
    let img = immeta::load_from_file(path).ok();
    img.map(|img| {
        let dim = img.dimensions();
        (Size::new(dim.width as i32, dim.height as i32), is_animated(&img))
    })
}

fn generate_archive_image_size(entry: &ArchiveEntry) -> Option<(Size, bool)> {
    let buf = &*entry.content;
    let img = immeta::load_from_buf(buf.as_slice()).ok();
    img.map(|img| {
        let dim = img.dimensions();
        (Size::new(dim.width as i32, dim.height as i32), is_animated(&img))
    })
}

fn is_animated(meta: &immeta::GenericMetadata) -> bool {
    if let Some(gif) = meta.as_ref::<immeta::markers::Gif>() {
        gif.is_animated()
    } else {
        false
    }
}
