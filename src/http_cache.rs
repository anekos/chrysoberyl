
use std::collections::VecDeque;
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;
use std::time::Duration;

use curl::easy::Easy as EasyCurl;
use curl;
use url::Url;

use app_path;
use entry::Meta;
use operation::{Operation, QueuedOperation};
use sorting_buffer::SortingBuffer;


type TID = usize;

#[derive(Clone)]
pub struct HttpCache {
    app_tx: Sender<Operation>,
    main_tx: Sender<Getter>,
    sorting_buffer: SortingBuffer<QueuedOperation>,
}

#[derive(Clone)]
struct Request {
    ticket: usize,
    url: String,
    cache_filepath: PathBuf,
    meta: Option<Meta>,
    force: bool
}


#[derive(Clone)]
enum Getter {
    Queue(String, PathBuf, Option<Meta>, bool),
    Done(usize, Request),
    Fail(usize, String, Request),
}


impl HttpCache {
    pub fn new(max_threads: u8, app_tx: Sender<Operation>, sorting_buffer: SortingBuffer<QueuedOperation>) -> HttpCache {
        let main_tx = main(max_threads, app_tx.clone(), sorting_buffer.clone());
        HttpCache { app_tx: app_tx, main_tx: main_tx, sorting_buffer: sorting_buffer  }
    }

    pub fn fetch(&mut self, url: String, meta: Option<Meta>, force: bool) -> Vec<QueuedOperation> {
        let filepath = generate_temporary_filename(&url);

        if filepath.exists() {
            self.sorting_buffer.push_with_reserve(
                QueuedOperation::PushHttpCache(filepath, url, meta, force))
        } else {
            self.main_tx.send(Getter::Queue(url, filepath, meta, force)).unwrap();
            vec![]
        }
    }
}


fn main(max_threads: u8, app_tx: Sender<Operation>, mut buffer: SortingBuffer<QueuedOperation>) -> Sender<Getter> {
    let (main_tx, main_rx) = channel();

    spawn(clone_army!([main_tx] move || {
        use self::Getter::*;

        let mut threads: Vec<Sender<Request>> = vec![];
        let mut waiting: Vec<TID> = vec![];
        let mut queued = VecDeque::<Request>::new();

        for thread_id in 0..max_threads as usize {
            threads.push(processor(thread_id, main_tx.clone()));
            waiting.push(thread_id);
        }

        while let Ok(it) = main_rx.recv() {
            match it {
                Queue(url, cache_filepath, meta, force) => {
                    let ticket = buffer.reserve();

                    let request = Request { ticket: ticket, url: url.clone(), cache_filepath: cache_filepath, meta: meta, force: force };

                    if let Some(worker) = waiting.pop() {
                        threads[worker].send(request).unwrap();
                    } else {
                        queued.push_back(request);
                        puts!("event" => "http/queue", "url" => o!(&url), "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));
                    }

                }
                Done(thread_id, request) => {
                    buffer.push(
                        request.ticket,
                        QueuedOperation::PushHttpCache(request.cache_filepath, request.url, request.meta, request.force));

                    app_tx.send(Operation::Pull).unwrap();

                    puts!("event" => "http/complete", "thread_id" => s!(thread_id), "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));

                    if let Some(next) = queued.pop_front() {
                        threads[thread_id].send(next).unwrap();
                    } else {
                        waiting.push(thread_id);
                    }
                }
                Fail(thread_id, err, request) => {
                    buffer.skip(request.ticket);

                    app_tx.send(Operation::Pull).unwrap();

                    puts_error!("at" => "http/get", "thread_id" => s!(thread_id), "reason" => err, "url" => o!(request.url), "queue" => s!(queued.len()), "buffer" => s!(buffer.len()), "waiting" => s!(waiting.len()));

                    if let Some(next) = queued.pop_front() {
                        threads[thread_id].send(next).unwrap();
                    } else {
                        waiting.push(thread_id);
                    }
                }
            }
        }
    }));

    main_tx
}

fn processor(thread_id: usize, main_tx: Sender<Getter>) -> Sender<Request> {
    let (getter_tx, getter_rx) = channel();

    spawn(move || {
        let mut curl = EasyCurl::new();

        // http://php.net/manual/ja/function.curl-setopt.php
        curl.low_speed_time(Duration::from_secs(10)).unwrap(); // CURLOPT_LOW_SPEED_TIME=10sec
        curl.low_speed_limit(1024).unwrap(); // CURLOPT_LOW_SPEED_LIMIT=1024
        // curl.timeout(Duration::from_secs(60)); // CURLOPT_TIMEOUT=60
        curl.follow_location(true).unwrap(); // Follow Redirection

        while let Ok(request) = getter_rx.recv() {
            let request: Request = request;

            puts!("event" => "http/get", "thread_id" => s!(thread_id), "url" => o!(&request.url));

            let mut buf = vec![];
            match curl_get(&mut curl, &request.url, &mut buf) {
                Ok(_) => {
                    let mut writer = BufWriter::new(File::create(&request.cache_filepath).unwrap());
                    writer.write_all(buf.as_slice()).unwrap();
                    main_tx.send(Getter::Done(thread_id, request)).unwrap();
                }
                Err(err) =>
                    main_tx.send(Getter::Fail(thread_id, format!("{}", err), request)).unwrap(),
            }
        }
    });

    getter_tx
}

fn curl_get(curl: &mut EasyCurl, url: &str, buf: &mut Vec<u8>) -> Result<(), curl::Error> {
    try!(curl.url(url));
    let mut transfer = curl.transfer();
    try! {
        transfer.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        })
    };
    transfer.perform()
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
