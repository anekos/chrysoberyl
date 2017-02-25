
use std::env::home_dir;
use std::fs::{File, create_dir_all};
use std::path::PathBuf;
use std::io::{BufWriter, Write, Read};
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;
use hyper::client::Client;
use hyper::client::response::Response;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use url::Url;

use output;
use operation::Operation;


#[derive(Clone)]
pub struct HttpCache {
    app_tx: Sender<Operation>,
    main_tx: Sender<(String, PathBuf)>
}

impl HttpCache {
    pub fn new(app_tx: Sender<Operation>) -> HttpCache {
        let main_tx = main(app_tx.clone());
        HttpCache { app_tx: app_tx, main_tx: main_tx }
    }

    pub fn fetch(&mut self, url: String) {
        let filepath = generate_temporary_filename(&url);

        if filepath.exists() {
            self.app_tx.send(Operation::PushFile(filepath)).unwrap();
        } else {
            self.main_tx.send((url, filepath)).unwrap();
        }

    }
}


fn main(app_tx: Sender<Operation>) -> Sender<(String, PathBuf)> {
    let (tx, rx) = channel();

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    spawn(move || {
        while let Ok((url, filepath)) = rx.recv() {
            output::puts1("HTTPGet", &url);

            match client.get(&url).send() {
                Ok(response) => {
                    write_to_file(&filepath, response);
                    app_tx.send(Operation::PushFile(filepath)).unwrap();
                }
                Err(err) => {
                    output::error(err);
                }
            }
        }
    });

    tx
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

    create_dir_all(&result.parent().unwrap()).unwrap();

    result
}
