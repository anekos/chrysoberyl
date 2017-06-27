
use std::path::Path;

use immeta;

use archive::ArchiveEntry;
use entry::EntryContent;
use lazy::Lazy;
use size::Size;



#[derive(Clone)]
pub struct EntryInfo {
    lazy_info: Lazy<LazyEntryInfo>,
    pub strict: StrictEntryInfo,
}

#[derive(Clone)]
pub struct LazyEntryInfo {
    pub dimensions: Option<Size>, // PDF makes None
    pub is_animated: bool,
}


#[derive(Clone)]
pub struct StrictEntryInfo {
    pub path: String, // local filepath or archive filepath or url
    pub extension: Option<String>,
    pub entry_type: &'static str,
}


impl EntryInfo {
    pub fn new(content: &EntryContent, path: &str) -> EntryInfo {
        use self::EntryContent::*;

        let extension = Path::new(path).extension().and_then(|it| it.to_str().map(|it| it.to_owned()));

        let entry_type: &'static str = match *content {
            File(_) => "file",
            Http(_, _) => "http",
            Archive(_, _) => "archive",
            Pdf(_, _) => "pdf",
        };

        EntryInfo {
            strict: StrictEntryInfo {
                entry_type: entry_type,
                extension: extension,
                path: o!(path),
            },
            lazy_info: Lazy::default()
        }

    }

    pub fn lazy(&mut self, content: &EntryContent) -> &LazyEntryInfo {
        self.lazy_info.get(|| LazyEntryInfo::new(content))

    }
}


impl LazyEntryInfo {
    pub fn new(content: &EntryContent) -> LazyEntryInfo {
        use entry::EntryContent::*;

        let meta = match *content {
            File(ref path) | Http(ref path, _) => generate_static_image_size(path),
            Archive(_, ref entry) => generate_archive_image_size(entry),
            Pdf(_, _) => None,
        };

        LazyEntryInfo {
            dimensions: meta.map(|it| it.0),
            is_animated: meta.map(|it| it.1).unwrap_or(false),
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
