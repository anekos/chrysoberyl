
use std::collections::VecDeque;
use std::env;
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;

use curl::easy::Easy as EasyCurl;
use curl;
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;
use url::Url;

use app_path;
use constant::env_name;
use entry::{Meta, EntryType};
use events::EventName;
use file_extension::get_entry_type_from_filename;
use mapping;
use operation::{Operation, QueuedOperation};
use sorting_buffer::SortingBuffer;
use utils::s;

pub mod curl_options;
mod db;

use self::curl_options::CurlOptions;



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
    options: CurlOptions,
}


#[derive(Clone)]
enum Getter {
    Queue(String, PathBuf, Option<Meta>, bool, Option<EntryType>),
    Done(usize, Request),
    UpdateCurlOptions(CurlOptions),
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
        let filepath = db::get_cached_filename(&url);

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

    pub fn update_curl_options(&self, options: CurlOptions) {
        self.main_tx.send(Getter::UpdateCurlOptions(options)).unwrap();
    }
}


fn main(max_threads: u8, app_tx: Sender<Operation>, mut buffer: SortingBuffer<QueuedOperation>) -> Sender<Getter> {
    let (main_tx, main_rx) = channel();

    spawn(clone_army!([main_tx] move || {
        use self::Getter::*;

        let mut threads: Vec<Sender<Request>> = vec![];
        let mut idles: Vec<TID> = vec![];
        let mut queued = VecDeque::<Request>::new();
        let mut options = CurlOptions::default();

        for thread_id in 0..max_threads as usize {
            threads.push(processor(thread_id, main_tx.clone()));
            idles.push(thread_id);
        }

        log_status(&SP::Initial, queued.len(), buffer.len(), idles.len(), threads.len());

        while let Ok(it) = main_rx.recv() {
            match it {
                Queue(url, cache_filepath, meta, force, entry_type) => {
                    let ticket = buffer.reserve();

                    let request = Request { ticket: ticket, url: url.clone(), cache_filepath: cache_filepath, meta: meta, force: force, entry_type: entry_type, options: options.clone() };

                    if let Some(worker) = idles.pop() {
                        threads[worker].send(request).unwrap();
                    } else {
                        queued.push_back(request);
                        log_status(&SP::Queue(url), queued.len(), buffer.len(), idles.len(), threads.len());
                    }
                }
                Done(thread_id, request) => {
                    buffer.push(
                        request.ticket,
                        make_queued_operation(request.cache_filepath, request.url, request.meta, request.force, request.entry_type));
                    app_tx.send(Operation::Pull).unwrap();
                    try_next(&app_tx, thread_id, queued.pop_front(), &mut threads, &mut idles);
                    log_status(&SP::Complete(thread_id), queued.len(), buffer.len(), idles.len(), threads.len());
                }
                Fail(thread_id, err, request) => {
                    buffer.skip(request.ticket);
                    app_tx.send(Operation::Pull).unwrap();
                    try_next(&app_tx, thread_id, queued.pop_front(), &mut threads, &mut idles);
                    log_status(&SP::Fail(thread_id, err, request.url), queued.len(), buffer.len(), idles.len(), threads.len());
                }
                UpdateCurlOptions(new_options) => {
                    println!("new_options: {:?}", new_options);
                    options = new_options;
                }
            }
        }
    }));

    main_tx
}

fn processor(thread_id: usize, main_tx: Sender<Getter>) -> Sender<Request> {
    let (getter_tx, getter_rx) = channel();

    spawn(move || {
        while let Ok(request) = getter_rx.recv() {
            let request: Request = request;
            let mut curl =  request.options.generate();

            puts!("event" => "remote/get", "thread_id" => s!(thread_id), "url" => o!(&request.url));

            let mut buf = vec![];
            let result = curl_get(&mut curl, &request.url, &mut buf).map_err(s).and_then(|_| {
                File::create(&request.cache_filepath).and_then(|file| {
                    let mut writer = BufWriter::new(file);
                    writer.write_all(buf.as_slice())
                }).map_err(s)
            });
            match result {
                Ok(_) => main_tx.send(Getter::Done(thread_id, request)).unwrap(),
                Err(err) => main_tx.send(Getter::Fail(thread_id, format!("{}", err), request)).unwrap(),
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

fn try_next(app_tx: &Sender<Operation>, thread_id: TID, next: Option<Request>, threads: &mut Vec<Sender<Request>>, idles: &mut Vec<TID>) {
    if let Some(next) = next {
        threads[thread_id].send(next).unwrap();
    } else {
        idles.push(thread_id);
    }

    if idles.len() == threads.len() {
        app_tx.send(Operation::Input(mapping::Input::Event(EventName::DownloadAll))).unwrap();
    }
}

fn log_status(sp: &SP, queues: usize, buffers: usize, idles: usize, threads: usize) {
    use self::SP::*;

    let (q, b, w, t) = (s!(queues), s!(buffers), s!(idles), s!((threads - idles)));
    match *sp {
        Initial => (),
        Queue(ref url) =>
            puts_event!("remote/queue", "url" => url, "queue" => q, "buffer" => b, "idles" => w),
        Complete(ref thread_id) =>
            puts_event!("remote/complete", "thread_id" => s!(thread_id), "queue" => q, "buffer" => b, "idles" => w),
        Fail(ref thread_id, ref error, ref url) =>
            puts_event!("remote/fail", "thread_id" => s!(thread_id), "reason" => error, "url" => url, "queue" => q, "buffer" => b, "idles" => w),
    }
    env::set_var(env_name("remote_queue"), q);
    env::set_var(env_name("remote_buffer"), b);
    env::set_var(env_name("remote_idles"), w);
    env::set_var(env_name("remote_thread"), t);
}
