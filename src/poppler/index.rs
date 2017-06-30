
use std::ffi::CStr;

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
            puts_error!("at" => "poppler/extract_action", "reason" => s!(err))
        }).ok()
    }
}
