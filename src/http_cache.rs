
use std::env::home_dir;
use std::fs::{File, create_dir_all};
use std::path::PathBuf;
use std::io::{BufWriter, Write, Read};
use hyper::client::Client;
use hyper::client::response::Response;
use hyper::Error;



#[derive(Clone)]
pub struct HttpCache {
}

impl HttpCache {
    pub fn new() -> HttpCache {
        HttpCache { }
    }

    pub fn get(&mut self, url: String) -> Result<String, Error> {
        self.get_path_buf(url).map(|path| path.to_str().unwrap().to_owned())
    }


    fn get_path_buf(&mut self, url: String) -> Result<PathBuf, Error> {
        let filepath = generate_temporary_filename(&url);

        if filepath.exists() {
            return Ok(filepath)
        }

        let client = Client::new();

        println!("HTTPGet\t{}", url);

        client.get(&url).send().map(|response| {
            write_to_file(&filepath, response);
            filepath
        })
    }
}


fn write_to_file(filepath: &PathBuf, mut response: Response) {
    let mut writer = BufWriter::new(File::create(filepath).unwrap());
    let mut data = vec![];
    response.read_to_end(&mut data).unwrap();
    writer.write(data.as_slice()).unwrap();
}

fn encode_filename(url: &str) -> String {
    let mut result = String::new();

    for c in url.chars() {
        match c {
            '_' => result.push_str("__"),
            '/' => result.push_str("_-"),
            _ => result.push(c)
        }
    }

    result
}

fn generate_temporary_filename(url: &str) -> PathBuf {
    let mut result = home_dir().unwrap();
    let filename = encode_filename(url);
    result.push(".cache");
    result.push(".chrysoberyl");
    result.push("url");
    create_dir_all(&result).unwrap();
    result.push(filename);
    result
}
