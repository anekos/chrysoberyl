
use std::env::home_dir;
use std::path::{PathBuf, Path};

use tldextract::{TldExtractor, TldOption};
use url::Url;

use utils::path_to_string;
use app_path;

#[cfg(test)] mod test;



lazy_static! {
    pub static ref EXTRACTOR: TldExtractor = {
        let mut cache_path = app_path::cache_dir("lib");
        cache_path.push("tldextract.json");

        let option = TldOption {
            cache_path: Some(path_to_string(&cache_path)),
            private_domains: false,
            update_local: true,
            naive_mode: false,
        };

        TldExtractor::new(option)
    };
}

pub fn shorten_url(url: Url, max: usize) -> String {
    let host = shorten_host(url.as_str()).unwrap_or_else(|| o!(url.host_str().unwrap_or("")));
    let path = Path::new(url.path());

    let path_max = max.checked_sub(host.len()).unwrap_or(0);
    let path = shorten_path(&path, path_max);

    if path.starts_with('/') {
        format!("{}{}", host, path)
    } else {
        format!("{}/{}", host, path)
    }
}

pub fn shorten_host(host: &str) -> Option<String> {
    EXTRACTOR.extract(host, None).map(|it| it.domain).ok()
}

pub fn shorten_path<T: AsRef<Path>>(path: &T, max: usize) -> String {
    let mut path = path.as_ref().to_path_buf();

    if let Some(home) = home_dir() {
        if path.starts_with(&home) {
            let mut s = path_to_string(&path);
            s.drain(0..path_to_string(&home).len());
            path = Path::new(&format!("~{}", s)).to_path_buf()
        }
    }

    while max < len(&path) {
        if let Some(short) = pop_front(&path) {
            path = short;
        } else {
            break;
        }
    }

    path_to_string(&path)
}


fn pop_front<T: AsRef<Path>>(path: &T) -> Option<PathBuf> {
    let mut cs = path.as_ref().components();
    let result = cs.next().map(|_| cs.as_path().to_path_buf());
    cs.next().and_then(|_| result)
}


fn len<T: AsRef<Path>>(path: &T) -> usize {
    path.as_ref().to_str().map(|it| it.len()).unwrap_or(0)
}
