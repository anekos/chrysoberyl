
use std::collections::VecDeque;
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
use entry::{Meta, MetaSlice, new_meta};
use operation::Operation;
use sorting_buffer::SortingBuffer;


type TID = usize;

#[derive(Clone)]
pub struct HttpCache {
    app_tx: Sender<Operation>,
    main_tx: Sender<Getter>,
}

#[derive(Clone)]
struct Request {
    serial: usize,
    url: String,
    cache_filepath: PathBuf,
    meta: Meta
}


#[derive(Clone)]
enum Getter {
    Queue(String, PathBuf, Meta),
    Done(usize, Request),
    Fail(usize, String, Request),
    Flush,
}


impl HttpCache {
    pub fn new(max_threads: u8, app_tx: Sender<Operation>) -> HttpCache {
        let main_tx = main(max_threads, app_tx.clone());
        HttpCache { app_tx: app_tx, main_tx: main_tx }
    }

    pub fn fetch(&mut self, url: String, meta: &MetaSlice) {
        let filepath = generate_temporary_filename(&url);

        if filepath.exists() {
            self.app_tx.send(Operation::PushHttpCache(filepath, url, new_meta(meta))).unwrap();
        } else {
            self.main_tx.send(Getter::Queue(url, filepath, new_meta(meta))).unwrap();
        }
    }

    pub fn force_flush(&self) {
        self.main_tx.send(Getter::Flush).unwrap();
    }
}


fn main(max_threads: u8, app_tx: Sender<Operation>) -> Sender<Getter> {
    let (main_tx, main_rx) = channel();

    spawn(clone_army!([main_tx] move || {
        use self::Getter::*;

        let mut serial: usize = 0;
        let mut buffer: SortingBuffer<Request> = SortingBuffer::new(serial);
        let mut threads: Vec<Sender<Request>> = vec![];
        let mut waiting: Vec<TID> = vec![];
        let mut queued = VecDeque::<Request>::new();

        for thread_id in 0..max_threads as usize {
            threads.push(processor(thread_id, main_tx.clone()));
            waiting.push(thread_id);
        }

        while let Ok(it) = main_rx.recv() {
            match it {
                Queue(url, cache_filepath, meta) => {
                    let request = Request { serial: serial, url: url.clone(), cache_filepath: cache_filepath, meta: meta };
                    serial += 1;
                    if let Some(worker) = waiting.pop() {
                        threads[worker].send(request).unwrap();
                    } else {
                        queued.push_back(request);
                        puts!("event" => "http/queue", "url" => o!(&url), "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));
                    }

                }
                Done(thread_id, request) => {
                    buffer.push(request.serial, request);
                    while let Some(request) = buffer.pull() {
                        app_tx.send(Operation::PushHttpCache(request.cache_filepath, request.url, request.meta)).unwrap();
                    }
                    puts!("event" => "http/complete", "thread_id" => s!(thread_id), "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));

                    if let Some(next) = queued.pop_front() {
                        threads[thread_id].send(next).unwrap();
                    } else {
                        waiting.push(thread_id);
                    }
                }
                Fail(thread_id, err, request) => {
                    waiting.push(thread_id);
                    buffer.skip(request.serial);
                    puts_error!("at" => "http/get", "reason" => err, "url" => o!(request.url), "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));
                }
                Flush => {
                    puts!("event" => "http/flush/start", "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));
                    for request in buffer.force_flush() {
                        app_tx.send(Operation::PushHttpCache(request.cache_filepath, request.url, request.meta)).unwrap();
                    }
                    puts!("event" => "http/flush/done", "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));
                }
            }
        }
    }));

    main_tx
}

fn processor(thread_id: usize, main_tx: Sender<Getter>) -> Sender<Request> {
    let (getter_tx, getter_rx) = channel();

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    spawn(move || {
        while let Ok(request) = getter_rx.recv() {
            let request: Request = request;

            puts!("event" => "http/get", "thread_id" => s!(thread_id), "url" => o!(&request.url));

            match client.get(&request.url).send() {
                Ok(response) => {
                    write_to_file(&request.cache_filepath, response);
                    main_tx.send(Getter::Done(thread_id, request)).unwrap();
                }
                Err(err) => {
                    main_tx.send(Getter::Fail(thread_id, format!("{}", err), request)).unwrap();
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
    let path = url.path();
    if path.len() <= 1 { // e.g.  "http://example.com" "http://example.com/"
        result.push(format!("{}.png", url.host().unwrap()));
        create_dir_all(&result.parent().unwrap()).unwrap();
        result
    } else {
        result.push(format!("{}{}", url.host().unwrap(), url.path()));
        create_dir_all(&result.parent().unwrap()).unwrap();
        result
    }
}
