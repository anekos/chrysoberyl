
use std::ffi::CStr;

use crate::poppler::sys;

use crate::errors::ErrorKind;
use crate::size::{Size, Region};



#[derive(Debug)]
pub struct Action {
    pub title: Option<String>,
    pub page: usize,
}


pub fn extract_action(action: *const sys::action_t) -> Option<Action> {
    unsafe {
        let action_type = &(*action).action_type;

        if let sys::ActionType::GotoRemote = *action_type {
        } else {
            return None;
        }

        let dest = (*action).dest;

        if dest.is_null() {
            println!("dest is null");
            return None;
        }

        let page = (*dest).page;
        if page <= 0 {
            return None;
        }

        let title = (*action).title;
        let title = if title.is_null() {
            None
        } else {
            CStr::from_ptr(title).to_str().map(|title| o!(title)).map_err(|err| {
                puts_error!(ErrorKind::Standard(s!(err)), "at" => "poppler/extract_action")
            }).ok()
        };

        Some(Action {title, page: page as usize})
    }
}


pub fn new_region_on(r: &sys::rectangle_t, size: Size) -> Region {
    let (w, h) = (f64!(size.width), f64!(size.height));
    let (x1, x2) = if r.x1 < r.x2 { (r.x1, r.x2 ) } else { (r.x2, r.x1) };
    let (y1, y2) = if r.y1 < r.y2 { (r.y1, r.y2 ) } else { (r.y2, r.y1) };
    Region {
        left: x1 / w,
        top: 1.0 - (y2 / h),
        right: x2 / w,
        bottom: 1.0 - (y1 / h),
    }
}
