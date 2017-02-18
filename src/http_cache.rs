
use std::env::home_dir;
use std::fs::{File, create_dir_all};
use std::path::PathBuf;
use std::io::{BufWriter, Write, Read};
use hyper::client::Client;
use hyper::client::response::Response;
use hyper::Error;
use url::Url;



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

fn generate_temporary_filename(url: &str) -> PathBuf {

    let mut result = home_dir().unwrap();
    result.push(".cache");
    result.push("chrysoberyl");
    result.push("http");

    {
        let url = Url::parse(url).unwrap();
        result.push(format!("{}{}", url.host().unwrap(), url.path()));
    }

    println!("{:?}", result);

    create_dir_all(&result.parent().unwrap()).unwrap();

    result
}
