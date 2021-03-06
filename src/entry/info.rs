
use std::borrow::ToOwned;
use std::fs::{metadata, Metadata};
use std::path::Path;
use std::time::SystemTime;

use log::info;

use crate::entry::EntryContent;
use crate::lazy::Lazy;
use crate::size::Size;
use crate::util::path::path_to_str;



pub struct EntryInfo {
    lazy_info: Lazy<LazyEntryInfo>,
    pub strict: StrictEntryInfo,
}

pub struct LazyEntryInfo {
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub dimensions: Option<Size>, // PDF makes None
    pub file_size: Option<u64>,
    pub is_animated: bool,
    pub modified: Option<SystemTime>,
    pub valid: bool,
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

        let extension = Path::new(path).extension().and_then(|it| it.to_str().map(ToOwned::to_owned));

        let entry_type: &'static str = match *content {
            Image(_) => "image",
            Archive(_, _) => "archive",
            Pdf(_, _) => "pdf",
            Memory(_, _) => "memory",
            Message(_) => "message",
        };

        let name: String = match *content {
            Image(ref path) => o!(path_to_str(path)),
            Archive(_, ref entry) => entry.name.clone(),
            Memory(_, ref hash) => hash.clone(),
            Pdf(ref path, _) => o!(path_to_str(&**path)),
            Message(ref message) => s!(message),
        };

        EntryInfo {
            strict: StrictEntryInfo {
                entry_type,
                extension,
                path: o!(path),
                name,
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
        use crate::entry::EntryContent::*;

        info!("LazyEntryInfo::new");

        let size_anim = match *content {
            Image(ref path) => generate_static_image_size(path),
            Archive(_, ref entry) => generate_archive_image_size(&entry.content),
            Memory(ref content, _) => generate_archive_image_size(content),
            Pdf(_, _) | Message(_) => None,
        };

        let valid = match *content {
            Image(_) | Archive(_, _) | Memory(_, _) => size_anim.is_some(),
            Pdf(_, _) => true,
            Message(_) => false,
        };

        let file_size = if let Memory(ref content, _) = *content {
            Some(content.len() as u64)
        } else {
            None
        };

        let file_meta = content.local_file_path().and_then(|ref it| metadata(it).ok());

        LazyEntryInfo {
            accessed: file_meta.as_ref().and_then(|it| it.accessed().ok()),
            created: file_meta.as_ref().and_then(|it| it.created().ok()),
            dimensions: size_anim.map(|it| it.0),
            file_size: file_size.or_else(|| file_meta.as_ref().map(Metadata::len)),
            is_animated: size_anim.map(|it| it.1).unwrap_or(false),
            modified: file_meta.as_ref().and_then(|it| it.modified().ok()),
            valid,
        }
    }
}


fn generate_static_image_size(path: &Path) -> Option<(Size, bool)> {
    let img = image_meta::load_from_file(path).ok();
    img.map(|img| {
        (Size::from(&img.dimensions), img.is_animation())
    })
}

fn generate_archive_image_size(buf: &[u8]) -> Option<(Size, bool)> {
    let img = image_meta::load_from_buf(buf).ok();
    img.map(|img| {
        (Size::from(&img.dimensions), img.is_animation())
    })
}
