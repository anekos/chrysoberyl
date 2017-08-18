
use std::fs::{File, create_dir_all};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use md5;
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;
use url::Url;

use app_path;



lazy_static! {
    static ref DB: Arc<r2d2::Pool<SqliteConnectionManager>> = {
        let mut path = app_path::cache_dir("remote");
        path.push("db.sqlite");
        let manager = SqliteConnectionManager::new(path);
        let config = r2d2::Config::builder().pool_size(16).build();
        let db = r2d2::Pool::new(config, manager).unwrap();
        {
            let conn = db.get().unwrap();
            let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='caches'").unwrap();
            let mut rows = stmt.query(&[]).unwrap();
            if !rows.next().is_some() {
                conn.execute(
                    "CREATE TABLE cache (url varchar, filename varchar, created_at varchar(19) default (datetime()));",
                    &[]).unwrap();
            }
        }
        return Arc::new(db);
    };
}


pub fn get_cached_filename(url: &str) -> PathBuf {
    let select = "SELECT filename FROM cache WHERE url = $1";

    let conn = &(*DB.get().unwrap());
    let mut stmt = conn.prepare(select).unwrap();

    match stmt.query_map(&[&url], |row| row.get(0)).unwrap().next() {
        Some(Ok(ref found)) => {
            let found: &String = found;
            Path::new(found).to_path_buf()
        },
        _ => {
            let result = generate_cache_filename(url);
            result
        }
    }
}

fn generate_cache_filename(url: &str) -> PathBuf {
    let mut result = app_path::cache_dir("remote");
    let url = Url::parse(url).unwrap();
    let host = url.host().unwrap();

    match url.path_segments() {
        Some(segs) => {
            let len = segs.clone().count();
            result.push(s!(host));
            for seg in segs.enumerate().map(|(i, it)| fix_path_segment(it, i == len - 1)) {
                result.push(seg);
            }
        },
        None => {
            result.push(format!("{}.png", url.host().unwrap()));
        }
    }

    create_dir_all(&result.parent().unwrap()).unwrap();
    result
}

fn fix_path_segment(s: &str, last: bool) -> String {
    if s.len() > 32 {
        if last {
            let ext = Path::new(s).extension().and_then(|it| it.to_str()).unwrap_or("");
            format!("{:x}.{}", md5::compute(s.as_bytes()), ext)
        } else {
            format!("{:x}", md5::compute(s.as_bytes()))
        }
    } else {
        o!(s)
    }
}


