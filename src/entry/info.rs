
use std::fs::metadata;
use std::path::Path;
use std::time::SystemTime;

use immeta;

use entry::EntryContent;
use lazy::Lazy;
use size::Size;
use util::path::path_to_str;



pub struct EntryInfo {
    lazy_info: Lazy<LazyEntryInfo>,
    pub strict: StrictEntryInfo,
}

pub struct LazyEntryInfo {
    pub dimensions: Option<Size>, // PDF makes None
    pub is_animated: bool,
    pub file_size: Option<u64>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
}


pub struct StrictEntryInfo {
    pub path: String, // local filepath, archive filepath or url
    pub name: String, // local filepath, inner filename in archive filepath or url
    pub extension: Option<String>,
    pub entry_type: &'static str,
    pub archive_page: i64, // page number in archive (1 origin)
}


impl EntryInfo {
    pub fn new(content: &EntryContent, path: &str, archive_page: usize) -> EntryInfo {
        use self::EntryContent::*;

        let extension = Path::new(path).extension().and_then(|it| it.to_str().map(|it| it.to_owned()));

        let entry_type: &'static str = match *content {
            Image(_) => "image",
            Archive(_, _) => "archive",
            Pdf(_, _) => "pdf",
            Memory(_, _) => "memory",
        };

        let name: String = match *content {
            Image(ref path) => o!(path_to_str(path)),
            Archive(_, ref entry) => entry.name.clone(),
            Memory(_, ref hash) => hash.clone(),
            Pdf(ref path, _) => o!(path_to_str(&**path)),
        };

        EntryInfo {
            strict: StrictEntryInfo {
                entry_type: entry_type,
                extension: extension,
                path: o!(path),
                name: name,
                archive_page: archive_page as i64,
            },
            lazy_info: Lazy::new()
        }
    }

    pub fn lazy<T, F>(&self, content: &EntryContent, get: F) -> T where F: FnOnce(&LazyEntryInfo) -> T {
        self.lazy_info.get(|| LazyEntryInfo::new(content), get)
    }
}


impl LazyEntryInfo {
    pub fn new(content: &EntryContent) -> LazyEntryInfo {
        use entry::EntryContent::*;

        info!("LazyEntryInfo::new");

        let size_anim = match *content {
            Image(ref path) => generate_static_image_size(path),
            Archive(_, ref entry) => generate_archive_image_size(&entry.content),
            Memory(ref content, _) => generate_archive_image_size(content),
            Pdf(_, _) => None,
        };

        let file_size = if let Memory(ref content, _) = *content {
            Some(content.len() as u64)
        } else {
            None
        };

        let file_meta = content.local_file_path().and_then(|ref it| metadata(it).ok());

        LazyEntryInfo {
            dimensions: size_anim.map(|it| it.0),
            is_animated: size_anim.map(|it| it.1).unwrap_or(false),
            file_size: file_size.or_else(|| file_meta.as_ref().map(|it| it.len())),
            accessed: file_meta.as_ref().and_then(|it| it.accessed().ok()),
            created: file_meta.as_ref().and_then(|it| it.created().ok()),
            modified: file_meta.as_ref().and_then(|it| it.modified().ok()),
        }
    }
}


fn generate_static_image_size(path: &Path) -> Option<(Size, bool)> {
    let img = immeta::load_from_file(path).ok();
    img.map(|img| {
        let dim = img.dimensions();
        (Size::new(dim.width as i32, dim.height as i32), is_animated(&img))
    })
}

fn generate_archive_image_size(buf: &[u8]) -> Option<(Size, bool)> {
    let img = immeta::load_from_buf(buf).ok();
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
