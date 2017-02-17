
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::{BufWriter, Write, Read};
use hyper::client::Client;
use hyper::client::response::Response;
use hyper::Error;
use mktemp::Temp;



pub struct HttpCache {
    cache: HashMap<String, PathBuf>, // URL to filepath
}

impl HttpCache {
    pub fn new() -> HttpCache {
        HttpCache { cache: HashMap::new() }
    }

    pub fn get(&mut self, url: String) -> Result<PathBuf, Error> {
        if let Some(filepath) = self.cache.get(&url) {
           return  Ok(filepath.clone())
        }

        let client = Client::new();

        client.get(&url).send().map(|response| {
            let filepath = write_to_file(&url, response);
            self.cache.insert(url, filepath.clone());
            filepath
        })
    }
}


fn write_to_file(url: &str, mut response: Response) -> PathBuf {
    let filepath = generate_temporary_filename(url);

    let mut writer = BufWriter::new(File::create(&filepath).unwrap());
    let mut data = vec![];
    response.read_to_end(&mut data).unwrap();
    writer.write(data.as_slice()).unwrap();

    filepath
}


fn generate_temporary_filename(url: &str) -> PathBuf {
    let extension = Path::new(url).extension().unwrap();
    let mut result = Temp::new_file().unwrap().to_path_buf();
    result.set_extension(extension);
    result
}
