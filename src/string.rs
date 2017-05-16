
use std::borrow::Cow;
use std::fmt;

use shell_escape;

use size::FitTo;
use state::{States, ScalingMethod};



pub fn stringify_states(st: &States) -> String {
    let mut result = o!("");

    result.push_str(&format!("@set status-bar {}\n", b2s(st.status_bar)));
    result.push_str(&format!("@set auto-paging {}\n", b2s(st.auto_paging)));
    result.push_str(&format!("@set reverse {}\n", b2s(st.auto_paging)));
    result.push_str(&format!("@set status-format {}\n", escape(&st.status_format.0)));
    result.push_str(&format!("@set pre-render {}\n", b2s(st.pre_fetch.enabled)));
    result.push_str(&format!("@set pre-render-limit {}\n", st.pre_fetch.limit_of_items));
    result.push_str(&format!("@set pre-render-pages {}\n", st.pre_fetch.page_size));
    result.push_str(&format!("@set fit-to {}\n", st.drawing.fit_to));
    result.push_str(&format!("@set scaling {}\n", st.drawing.scaling));
    // result.push_str(format!("@set clip {}\n", st.drawing.fit_to);

    result
}


fn b2s(b: bool) -> &'static str {
    if b {
        "true"
    } else {
        "false"
    }
}


impl fmt::Display for FitTo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use size::FitTo::*;

        let result =
            match *self {
                Original => "original",
                OriginalOrCell => "original-or-cell",
                Width => "width",
                Height => "height",
                Cell => "cell",
            };

        write!(f, "{}", result)
    }
}


impl fmt::Display for ScalingMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use gdk_pixbuf::InterpType::*;

        let result = match self.0 {
            Nearest => "nearest",
            Tiles => "tiles",
            Bilinear => "bilinear",
            Hyper => "hyper",
        };
        write!(f, "{}", result)
    }
}


fn escape(s: &str) -> String {
    let s = Cow::from(o!(s));
    shell_escape::escape(s).into_owned()
}
