
use std::path::Path;
use std::default::Default;

use immeta;
use regex::Regex;

use archive::ArchiveEntry;
use entry::{Entry, EntryInfo, EntryContent};
use size::Size;



#[derive(Clone, Debug)]
pub struct Condition {
    pub min_width: Option<i32>,
    pub min_height: Option<i32>,
    pub max_width: Option<i32>,
    pub max_height: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub min_dimensions: Option<i32>,
    pub max_dimensions: Option<i32>,
    pub extensions: Vec<String>,
    pub path: Option<Regex>,
}

impl Condition {
    pub fn is_valid(&self, entry: &mut Entry) -> bool {
        if !self.is_empty_for_info() {
            let info = get_info(entry);

            if let Some(size) = info.size {
                if let Some(min_width) = self.min_width {
                    if size.width < min_width {
                        return false;
                    }
                }
                if let Some(min_height) = self.min_height {
                    if size.height < min_height {
                        return false;
                    }
                }
                if let Some(max_width) = self.max_width {
                    if max_width < size.width {
                        return false;
                    }
                }
                if let Some(max_height) = self.max_height {
                    if max_height < size.height {
                        return false;
                    }
                }
                let dims = size.height * size.width;
                if let Some(min_dimensions) = self.min_dimensions {
                    if dims < min_dimensions {
                        return false;
                    }
                }
                if let Some(max_dimensions) = self.max_dimensions {
                    if max_dimensions < dims {
                        return false;
                    }
                }
            }
        }

        if 0 < self.extensions.len() {
            if !is_valid_extension(&entry.key.1, &self.extensions) {
                return false;
            }
        }

        if let Some(ref path) = self.path {
            if !path.is_match(&entry.key.1) {
                return false;
            }
        }

        true
    }

    pub fn optionize(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    fn is_empty_for_info(&self) -> bool {
        !(self.min_width.is_some() ||
          self.min_height.is_some() ||
          self.max_width.is_some() ||
          self.max_height.is_some() ||
          self.width.is_some() ||
          self.height.is_some() ||
          self.min_dimensions.is_some() ||
          self.max_dimensions.is_some())
    }

    fn is_empty(&self) -> bool {
        !(!self.extensions.is_empty() ||
          self.path.is_some()) &&
            self.is_empty_for_info()
    }
}

impl Default for Condition {
    fn default() -> Self {
        Condition {
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            width: None,
            height: None,
            min_dimensions: None,
            max_dimensions: None,
            extensions: vec![],
            path: None,
        }
    }
}


pub fn get_info(entry: &mut Entry) -> &EntryInfo {
    let info = &mut entry.info;
    let content = &entry.content;

    info.get(|| generate_info(&content))
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

fn is_valid_extension(path: &str, extensions: &[String]) -> bool {
    if_let_some!(ext = Path::new(path).extension(), true);
    let ext = ext.to_str().unwrap().to_lowercase();

    for extension in extensions {
        if &ext == extension {
            return true
        }
    }

    false
}
