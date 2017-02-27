
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

static THREADS: usize = 3;



#[derive(Clone)]
pub struct HttpCache {
    app_tx: Sender<Operation>,
    main_tx: Sender<Getter>
}

#[derive(Clone)]
struct Request {
    url: String,
    cache_filepath: PathBuf
}


#[derive(Clone)]
enum Getter {
    Get(Request),
    Done(usize, Request),
    Fail(usize, String)
}


impl HttpCache {
    pub fn new(app_tx: Sender<Operation>) -> HttpCache {
        let main_tx = getter_main(app_tx.clone());
        HttpCache { app_tx: app_tx, main_tx: main_tx }
    }

    pub fn fetch(&mut self, url: String) {
        let filepath = generate_temporary_filename(&url);

        if filepath.exists() {
            self.app_tx.send(Operation::PushFile(filepath)).unwrap();
        } else {
            self.main_tx.send(Getter::Get(Request { url: url, cache_filepath: filepath })).unwrap();
        }

    }
}


fn getter_main(app_tx: Sender<Operation>) -> Sender<Getter> {
    let (main_tx, main_rx) = channel();

    spawn(clone_army!([main_tx] move || {
        use self::Getter::*;

        let mut stacks: Vec<usize> = vec![];
        let mut threads: Vec<Sender<Request>> = vec![];

        for index in 0..THREADS {
            stacks.push(0);
            threads.push(getter_thread(index, main_tx.clone()));
        }

        while let Ok(it) = main_rx.recv() {
            match it {
                Get(request) => {
                    output::puts1("HTTPGet", &request.url);

                    let mut min_index = 0;
                    let mut min_stack = <usize>::max_value();
                    for (index, stack) in stacks.iter().enumerate() {
                        if *stack < min_stack {
                            min_index = index;
                            min_stack = *stack;
                        }
                    }

                    let mut stack = stacks.get_mut(min_index).unwrap();
                    *stack += 1;
                    threads[min_index].send(request).unwrap();
                }
                Done(index, request) => {
                    app_tx.send(Operation::PushFile(request.cache_filepath)).unwrap();
                    let mut stack = stacks.get_mut(index).unwrap();
                    *stack -= 1;
                    output::puts1("HTTPDone", index);
                }
                Fail(index, err) => {
                    let mut stack = stacks.get_mut(index).unwrap();
                    *stack -= 1;
                    output::puts1("HTTPFail", err);
                }
            }
        }
    }));

    main_tx
}

fn getter_thread(id: usize, main_tx: Sender<Getter>) -> Sender<Request> {
    let (getter_tx, getter_rx) = channel();

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    spawn(move || {
        while let Ok(Request { url, cache_filepath }) = getter_rx.recv() {

            match client.get(&url).send() {
                Ok(response) => {
                    write_to_file(&cache_filepath, response);
                    main_tx.send(Getter::Done(id, Request { url: url, cache_filepath: cache_filepath })).unwrap();
                }
                Err(err) => {
                    main_tx.send(Getter::Fail(id, format!("{}", err))).unwrap();
                }
            }
        }
    });

    getter_tx
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
