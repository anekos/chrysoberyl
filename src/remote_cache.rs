
use std::collections::VecDeque;
use std::env;
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::{PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;
use std::time::Duration;

use curl::easy::Easy as EasyCurl;
use curl;
use url::Url;

use app_path;
use constant::env_name;
use entry::{Meta, EntryType};
use events::EventName;
use file_extension::get_entry_type_from_filename;
use mapping;
use operation::{Operation, QueuedOperation};
use sorting_buffer::SortingBuffer;



type TID = usize;

#[derive(Clone)]
pub struct RemoteCache {
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
    force: bool,
    entry_type: Option<EntryType>,
}


#[derive(Clone)]
enum Getter {
    Queue(String, PathBuf, Option<Meta>, bool, Option<EntryType>),
    Done(usize, Request),
    Fail(usize, String, Request),
}

// Status Paramter
enum SP {
    Initial,
    Queue(String),
    Complete(usize),
    Fail(usize, String, String),
}


impl RemoteCache {
    pub fn new(max_threads: u8, app_tx: Sender<Operation>, sorting_buffer: SortingBuffer<QueuedOperation>) -> Self {
        let main_tx = main(max_threads, app_tx.clone(), sorting_buffer.clone());
        RemoteCache { app_tx: app_tx, main_tx: main_tx, sorting_buffer: sorting_buffer  }
    }

    pub fn fetch(&mut self, url: String, meta: Option<Meta>, force: bool, entry_type: Option<EntryType>) -> Vec<QueuedOperation> {
        let filepath = generate_temporary_filename(&url);

        if filepath.exists() {
            let result = self.sorting_buffer.push_with_reserve(
                make_queued_operation(filepath, url, meta, force, entry_type));
            self.update_sorting_buffer_len();
            result
        } else {
            self.main_tx.send(Getter::Queue(url, filepath, meta, force, entry_type)).unwrap();
            vec![]
        }
    }

    pub fn update_sorting_buffer_len(&self) {
        env::set_var(env_name("remote_buffer"), s!(self.sorting_buffer.len()));
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

        log_status(SP::Initial, queued.len(), buffer.len(), waiting.len(), threads.len());

        while let Ok(it) = main_rx.recv() {
            match it {
                Queue(url, cache_filepath, meta, force, entry_type) => {
                    let ticket = buffer.reserve();

                    let request = Request { ticket: ticket, url: url.clone(), cache_filepath: cache_filepath, meta: meta, force: force, entry_type: entry_type };

                    if let Some(worker) = waiting.pop() {
                        threads[worker].send(request).unwrap();
                    } else {
                        queued.push_back(request);
                        log_status(SP::Queue(url), queued.len(), buffer.len(), waiting.len(), threads.len());
                    }
                }
                Done(thread_id, request) => {
                    buffer.push(
                        request.ticket,
                        make_queued_operation(request.cache_filepath, request.url, request.meta, request.force, request.entry_type));
                    app_tx.send(Operation::Pull).unwrap();
                    try_next(&app_tx, thread_id, queued.pop_front(), &mut threads, &mut waiting);
                    log_status(SP::Complete(thread_id), queued.len(), buffer.len(), waiting.len(), threads.len());
                }
                Fail(thread_id, err, request) => {
                    buffer.skip(request.ticket);
                    app_tx.send(Operation::Pull).unwrap();
                    try_next(&app_tx, thread_id, queued.pop_front(), &mut threads, &mut waiting);
                    log_status(SP::Fail(thread_id, err, request.url), queued.len(), buffer.len(), waiting.len(), threads.len());
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
        curl.connect_timeout(Duration::from_secs(10)).unwrap(); // CURLOPT_CONNECTTIMEOUT_MS
        curl.low_speed_time(Duration::from_secs(10)).unwrap();  // CURLOPT_LOW_SPEED_TIME=10sec
        curl.low_speed_limit(1024).unwrap();                    // CURLOPT_LOW_SPEED_LIMIT=1024
        curl.follow_location(true).unwrap();                    // Follow Redirection
        // curl.timeout(Duration::from_secs(60));               // CURLOPT_TIMEOUT=60

        while let Ok(request) = getter_rx.recv() {
            let request: Request = request;

            puts!("event" => "remote/get", "thread_id" => s!(thread_id), "url" => o!(&request.url));

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
    let mut result = app_path::cache_dir("remote");
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

fn make_queued_operation(file: PathBuf, url: String, meta: Option<Meta>, force: bool, entry_type: Option<EntryType>) -> QueuedOperation {
    let entry_type = entry_type.or_else(|| {
        get_entry_type_from_filename(&file)
    }).unwrap_or(EntryType::Image);

    match entry_type {
        EntryType::Image =>
            QueuedOperation::PushImage(file, meta, force, None, Some(url)),
        EntryType::Archive =>
            QueuedOperation::PushArchive(file, meta, force, Some(url)),
        EntryType::PDF =>
            QueuedOperation::PushPdf(file, meta, force,  Some(url)),
        _ =>
            not_implemented!(),
    }
}

fn try_next(app_tx: &Sender<Operation>, thread_id: TID, next: Option<Request>, threads: &mut Vec<Sender<Request>>, waiting: &mut Vec<TID>) {
    if let Some(next) = next {
        threads[thread_id].send(next).unwrap();
    } else {
        waiting.push(thread_id);
    }

    if waiting.len() == threads.len() {
        app_tx.send(Operation::Input(mapping::Input::Event(EventName::DownloadAll))).unwrap();
    }
}

fn log_status(sp: SP, queues: usize, buffers: usize, waitings: usize, threads: usize) {
    use self::SP::*;

    let (q, b, w, t) = (s!(queues), s!(buffers), s!(waitings), s!((threads - waitings)));
    match sp {
        Initial => (),
        Queue(ref url) =>
            puts_event!("remote/queue", "url" => url, "queue" => q, "buffer" => b, "waiting" => w),
        Complete(ref thread_id) =>
            puts_event!("remote/complete", "thread_id" => s!(thread_id), "queue" => q, "buffer" => b, "waiting" => w),
        Fail(ref thread_id, ref error, ref url) =>
            puts_event!("remote/fail", "thread_id" => s!(thread_id), "reason" => error, "url" => url, "queue" => q, "buffer" => b, "waiting" => w),
    }
    env::set_var(env_name("remote_queue"), q);
    env::set_var(env_name("remote_buffer"), b);
    env::set_var(env_name("remote_waiting"), w);
    env::set_var(env_name("remote_thread"), t);
}
