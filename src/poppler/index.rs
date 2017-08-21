
use std::default::Default;
use std::ffi::CStr;
use std::str::FromStr;

use poppler::sys;



#[derive(Debug)]
pub struct Index {
    pub entries: Vec<IndexEntry>,
}

#[derive(Debug)]
pub struct IndexEntry {
    pub title: String,
    pub page: usize,
    pub child: Option<Index>,
}

#[derive(Clone, Debug, Copy)]
pub enum Format {
    Indented,
    OneLine,
    TwoLines,
}



impl Index {
    pub fn new(iter: *const sys::page_index_iter_t) -> Self {
        let mut result = Index { entries: vec![] };

        unsafe {
            loop {
                let action = sys::poppler_index_iter_get_action(iter);
                if action.is_null() {
                    break;
                }

                if let Some(mut entry) = extract_action(action) {
                    let child = sys::poppler_index_iter_get_child(iter);
                    entry.child = if child.is_null() {
                        None
                    } else {
                        Some(Index::new(child))
                    };
                    result.entries.push(entry);
                }

                sys::poppler_action_free(action);

                if sys::poppler_index_iter_next(iter) == 0 {
                    break;
                }
            }
        }

        result
    }

    pub fn write(&self, fmt: &Format, separator: Option<&str>, out: &mut String) {
        use self::Format::*;

        match *fmt {
            Indented =>
                write_indented(self, separator.unwrap_or(" = "), 0, out),
            OneLine =>
                write_one_line(self, separator.unwrap_or("="), out),
            TwoLines =>
                write_two_lines(self, out),
        }
    }
}


fn extract_action(action: *const sys::action_t) -> Option<IndexEntry> {
    unsafe {
        let action_type = (*action).action_type;

        if action_type != 2 {
            return None;
        }

        let dest = (*action).dest;

        if dest.is_null() {
            return None;
        }

        let title = (*action).title;
        if title.is_null() {
            return None;
        }

        CStr::from_ptr(title).to_str().map(|title| {
            IndexEntry {
                title: o!(title),
                page: (*dest).page as usize,
                child: None,
            }
        }).map_err(|err| {
            puts_error!(s!(err), "at" => "poppler/extract_action")
        }).ok()
    }
}


impl Default for Format {
    fn default() -> Self {
        Format::Indented
    }
}

impl FromStr for Format {
    type Err = String;

    fn from_str(src: &str) -> Result<Self, String> {
        use self::Format::*;

        let result = match src {
            "1" | "one-line" | "one" | "o" => OneLine,
            "2" | "two-lines" | "two" | "t" => TwoLines,
            "indented" | "indent" | "i"  => Indented,
            _ => return Err(format!("Invalid format name: {}", src)),
        };
        Ok(result)
    }
}


fn write_indented(index: &Index, separator: &str, level: u8, out: &mut String) {
        let indent = "  ".repeat(level as usize);

        for entry in &index.entries {
            sprint!(out, &indent);
            sprintln!(out, "{:03}{}{}", entry.page, separator, entry.title);
            if let Some(ref child) = entry.child {
                write_indented(child, separator, level + 1, out);
            }
        }
}

fn write_one_line(index: &Index, separator: &str, out: &mut String) {
        for entry in &index.entries {
            sprintln!(out, "{}{}{}", entry.page, separator, entry.title);
            if let Some(ref child) = entry.child {
                write_one_line(child, separator, out);
            }
        }
}

fn write_two_lines(index: &Index, out: &mut String) {
        for entry in &index.entries {
            sprintln!(out, "{}", entry.page);
            sprintln!(out, "{}", entry.title);
            if let Some(ref child) = entry.child {
                write_two_lines(child, out);
            }
        }
}
