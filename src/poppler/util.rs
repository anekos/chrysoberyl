
use std::ffi::CStr;

use poppler::sys;

use errors::ChryError;
use size::{Size, Region};



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
            println!("type: {:?}", *action_type);
            return None;
        }

        let dest = (*action).dest;

        if dest.is_null() {
            println!("dest is null");
            return None;
        }

        let title = (*action).title;
        let title = if title.is_null() {
            None
        } else {
            CStr::from_ptr(title).to_str().map(|title| o!(title)).map_err(|err| {
                puts_error!(ChryError::Standard(s!(err)), "at" => "poppler/extract_action")
            }).ok()
        };

        Some(Action {title: title, page: (*dest).page as usize})
    }
}


pub fn new_region_on(pdf_region: &sys::rectangle_t, size: &Size) -> Region {
    let (w, h) = (f64!(size.width), f64!(size.height));
    Region {
        left: pdf_region.x1 / w,
        top: 1.0 - (pdf_region.y1 / h),
        right: pdf_region.x2 / w,
        bottom: 1.0 - (pdf_region.y2 / h),
    }
}
