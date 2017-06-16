
use std::path::Path;
use std::default::Default;

use immeta;

use entry::{Entry, EntryInfo, EntryContent};
use size::Size;



#[derive(Clone, Debug, PartialEq)]
pub struct Condition {
    pub min_width: Option<i32>,
    pub min_height: Option<i32>,
}

impl Condition {
    pub fn is_valid(&self, entry: &mut Entry) -> bool {
        println!("key: {:?}", entry.key);
        let info = get_info(entry);

        println!("condition: {:?}", self);

        if let Some(size) = info.size {
            println!("size: {:?}", size);
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

    fn is_empty(&self) -> bool {
        !(self.min_width.is_some() || self.min_height.is_some())
    }
}

impl Default for Condition {
    fn default() -> Self {
        Condition {
            min_width: None,
            min_height: None,
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
        _ =>
            EntryInfo { size: None } // TODO FIXME
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
