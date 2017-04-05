
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write, Read};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;

use hyper::client::Client;
use hyper::client::response::Response;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use url::Url;

use app_path;
use operation::Operation;
use sorting_buffer::SortingBuffer;



#[derive(Clone)]
pub struct HttpCache {
    app_tx: Sender<Operation>,
    main_tx: Sender<Getter>
}

#[derive(Clone)]
struct Request {
    serial: usize,
    url: String,
    cache_filepath: PathBuf
}


#[derive(Clone)]
enum Getter {
    Queue(String, PathBuf),
    Done(usize, Request),
    Fail(usize, String, Request)
}


impl HttpCache {
    pub fn new(max_threads: u8, app_tx: Sender<Operation>) -> HttpCache {
        let main_tx = getter_main(max_threads, app_tx.clone());
        HttpCache { app_tx: app_tx, main_tx: main_tx }
    }

    pub fn fetch(&mut self, url: String) {
        let filepath = generate_temporary_filename(&url);

        if filepath.exists() {
            self.app_tx.send(Operation::PushHttpCache(filepath, url)).unwrap();
        } else {
            self.main_tx.send(Getter::Queue(url, filepath)).unwrap();
        }
    }
}


fn getter_main(max_threads: u8, app_tx: Sender<Operation>) -> Sender<Getter> {
    let (main_tx, main_rx) = channel();

    spawn(clone_army!([main_tx] move || {
        use self::Getter::*;

        let mut stacks: Vec<usize> = vec![];
        let mut threads: Vec<Sender<Request>> = vec![];
        let mut serial: usize = 0;
        let mut queued: usize = 0;
        let mut buffer: SortingBuffer<Request> = SortingBuffer::new(serial);

        for index in 0..max_threads as usize {
            stacks.push(0);
            threads.push(getter_thread(index, main_tx.clone()));
        }

        while let Ok(it) = main_rx.recv() {
            match it {
                Queue(url, cache_filepath) => {
                    let mut min_index = 0;
                    let mut min_stack = <usize>::max_value();
                    for (index, stack) in stacks.iter().enumerate() {
                        if *stack < min_stack {
                            min_index = index;
                            min_stack = *stack;
                        }
                    }

                    queued += 1;

                    let mut stack = stacks.get_mut(min_index).unwrap();
                    *stack += 1;

                    let request = Request { serial: serial, url: url.clone(), cache_filepath: cache_filepath };
                    serial += 1;

                    threads[min_index].send(request).unwrap();

                    puts!("event" => "HTTP", "state" => "get", "thread_id" => s!(min_index), "url" => o!(&url), "queue" => s!(queued), "buffer" => s!(buffer.len()));
                }
                Done(index, request) => {
                    queued -= 1;
                    let mut stack = stacks.get_mut(index).unwrap();
                    *stack -= 1;

                    buffer.push(request.serial, request);

                    while let Some(request) = buffer.pull() {
                        app_tx.send(Operation::PushHttpCache(request.cache_filepath, request.url)).unwrap();
                    }

                    puts!("event" => "HTTP", "state" => "done", "thread_id" => s!(index), "queue" => s!(queued), "buffer" => s!(buffer.len()));
                }
                Fail(index, err, request) => {
                    queued -= 1;
                    let mut stack = stacks.get_mut(index).unwrap();
                    *stack -= 1;
                    buffer.skip(request.serial);
                    puts_error!("at" => "HTTP/Get", "reason" => err, "url" => o!(request.url), "queue" => s!(queued), "buffer" => s!(buffer.len()));
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
        while let Ok(request) = getter_rx.recv() {
            let request: Request = request;

            match client.get(&request.url).send() {
                Ok(response) => {
                    write_to_file(&request.cache_filepath, response);
                    main_tx.send(Getter::Done(id, request)).unwrap();
                }
                Err(err) => {
                    main_tx.send(Getter::Fail(id, format!("{}", err), request)).unwrap();
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
    writer.write_all(data.as_slice()).unwrap();
}

fn generate_temporary_filename(url: &str) -> PathBuf {
    let mut result = app_path::cache_dir("http");
    let url = Url::parse(url).unwrap();
    result.push(format!("{}{}", url.host().unwrap(), url.path()));
    create_dir_all(&result.parent().unwrap()).unwrap();
    result
}
