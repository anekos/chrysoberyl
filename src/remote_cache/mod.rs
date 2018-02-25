
use std::cmp::Ordering;
use std::collections::{BTreeSet, VecDeque};
use std::env;
use std::error::Error;
use std::fs::{self, File, create_dir_all};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;

use curl::easy::Easy as EasyCurl;
use filetime::{FileTime, set_file_times};
use md5;
use time;
use url::Url;

use app_path;
use constant::env_name;
use entry::{Meta, EntryType};
use errors::ChryError;
use events::EventName;
use file_extension::get_entry_type_from_filename;
use mapping;
use operation::{Operation, QueuedOperation};
use sorting_buffer::SortingBuffer;

pub mod curl_options;

use self::curl_options::CurlOptions;



type TID = usize;

pub struct RemoteCache {
    main_tx: Sender<Getter>,
    sorting_buffer: SortingBuffer<QueuedOperation>,
    pub state: Arc<Mutex<State>>,
    pub do_update_atime: bool,
}


#[derive(Default)]
pub struct State {
    idles: Vec<TID>,
    processing: BTreeSet<Request>,
    queued: VecDeque<Request>,
    threads: Vec<Sender<Request>>,
}

#[derive(Clone)]
pub struct Request {
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
    Process(String),
    Queue(String),
    Complete(usize),
    Fail(usize, String, String),
}


impl RemoteCache {
    pub fn new(max_threads: u8, app_tx: Sender<Operation>, sorting_buffer: SortingBuffer<QueuedOperation>) -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let main_tx = main(max_threads, app_tx, sorting_buffer.clone(), state.clone());
        RemoteCache { main_tx, sorting_buffer, do_update_atime: false, state }
    }

    pub fn fetch(&mut self, url: String, meta: Option<Meta>, force: bool, entry_type: Option<EntryType>) -> Vec<QueuedOperation> {
        if_let_ok!(filepath = generate_temporary_filename(&url), |err: Box<Error>| {
            puts_error!(err, "at" => "generate_temporary_filename");
            vec![]
        });

        if filepath.exists() {
            if self.do_update_atime {
                if let Err(e) = update_atime(&filepath) {
                    puts_error!(e, "at" => "update_atime");
                }
            }
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


fn main(max_threads: u8, app_tx: Sender<Operation>, mut buffer: SortingBuffer<QueuedOperation>, state: Arc<Mutex<State>>) -> Sender<Getter> {
    let (main_tx, main_rx) = channel();

    spawn(clone_army!([main_tx] move || {
        use self::Getter::*;

        let mut options = CurlOptions::default();

        {
            let mut state = state.lock().unwrap();
            for thread_id in 0..max_threads as usize {
                state.threads.push(processor(thread_id, main_tx.clone()));
                state.idles.push(thread_id);
            }
            log_status(&SP::Initial, &state, buffer.len());
        }


        while let Ok(it) = main_rx.recv() {
            match it {
                Queue(url, cache_filepath, meta, force, entry_type) => {
                    let mut state = state.lock().unwrap();
                    let ticket = buffer.reserve();

                    let request = Request { ticket: ticket, url: url.clone(), cache_filepath: cache_filepath, meta: meta, force: force, entry_type: entry_type, options: options.clone() };

                    if let Some(worker) = state.idles.pop() {
                        state.processing.insert(request.clone());
                        state.threads[worker].send(request).unwrap();
                        log_status(&SP::Process(url), &state, buffer.len());
                    } else {
                        state.queued.push_back(request);
                        log_status(&SP::Queue(url), &state, buffer.len());
                    }
                }
                Done(thread_id, request) => {
                    let mut state = state.lock().unwrap();
                    state.processing.remove(&request);
                    buffer.push(
                        request.ticket,
                        make_queued_operation(request.cache_filepath, request.url, request.meta, request.force, request.entry_type));
                    app_tx.send(Operation::Pull).unwrap();
                    try_next(&app_tx, thread_id, &mut state);
                    log_status(&SP::Complete(thread_id), &state, buffer.len());
                }
                Fail(thread_id, err, request) => {
                    let mut state = state.lock().unwrap();
                    state.processing.remove(&request);
                    buffer.skip(request.ticket);
                    app_tx.send(Operation::Pull).unwrap();
                    try_next(&app_tx, thread_id, &mut state);
                    log_status(&SP::Fail(thread_id, err, request.url), &state, buffer.len());
                }
                UpdateCurlOptions(new_options) => {
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

            match http_save(&mut curl, &request.url, &request.cache_filepath) {
                Ok(_) => main_tx.send(Getter::Done(thread_id, request)).unwrap(),
                Err(err) => main_tx.send(Getter::Fail(thread_id, s!(err), request)).unwrap(),
            }
        }
    });

    getter_tx
}

fn http_save<T: AsRef<Path>>(curl: &mut EasyCurl, url: &str, cache_filepath: &T) -> Result<(), Box<Error>> {
    let mut buf = vec![];
    curl_get(curl, url, &mut buf)?;
    File::create(cache_filepath).and_then(|file| {
        let mut writer = BufWriter::new(file);
        writer.write_all(buf.as_slice())
    })?;
    Ok(())
}

fn curl_get(curl: &mut EasyCurl, url: &str, buf: &mut Vec<u8>) -> Result<(), Box<Error>> {
    curl.url(url)?;
    let mut transfer = curl.transfer();
    transfer.write_function(|data| {
        buf.extend_from_slice(data);
        Ok(data.len())
    })?;
    transfer.perform()?;
    Ok(())
}

fn fix_path_segment(s: &str, last: bool) -> String {
    if s.len() > 32 {
        if last {
            let ext = Path::new(s).extension().and_then(|it| it.to_str()).unwrap_or("");
            format!("{:x}.{}", md5::compute(s.as_bytes()), ext)
        } else {
            format!("{:x}", md5::compute(s.as_bytes()))
        }
    } else {
        o!(s)
    }
}

fn generate_temporary_filename(url: &str) -> Result<PathBuf, Box<Error>> {
    let mut result = app_path::cache_dir("remote");
    let url = Url::parse(url)?;
    let host = url.host().ok_or_else(|| format!("URL does not have `host`: {}", url))?;

    match url.path_segments() {
        Some(segs) => {
            let len = segs.clone().count();
            result.push(s!(host));
            for seg in segs.enumerate().map(|(i, it)| fix_path_segment(it, i == len - 1)) {
                result.push(seg);
            }
        },
        None => {
            result.push(format!("{}.png", url.host().unwrap()));
        }
    }

    create_dir_all(&result.parent().unwrap()).unwrap();
    Ok(result)
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

fn try_next(app_tx: &Sender<Operation>, thread_id: TID, state: &mut State) {
    if let Some(next) = state.queued.pop_front() {
        state.threads[thread_id].send(next).unwrap();
    } else {
        state.idles.push(thread_id);
    }

    if state.idles.len() == state.threads.len() {
        app_tx.send(Operation::Input(mapping::Input::Event(EventName::DownloadAll))).unwrap();
    }
}

fn log_status(sp: &SP, state: &State, buffers: usize) {
    use self::SP::*;

    let idles = state.idles.len();
    let (q, b, w, t) = (s!(state.queued.len()), s!(buffers), s!(idles), s!((state.threads.len() - idles)));
    match *sp {
        Initial => (),
        Process(ref url) =>
            puts_event!("remote/process", "url" => url, "queue" => q, "buffer" => b, "idles" => w),
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


fn update_atime<T: AsRef<Path>>(path: &T) -> Result<(), ChryError> {
    let meta = try!(fs::metadata(path));
    let ts = time::now().to_timespec();
    let mtime = FileTime::from_last_modification_time(&meta);
    let atime = FileTime::from_seconds_since_1970(ts.sec as u64, ts.nsec as u32);
    set_file_times(path, atime, mtime).map_err(|it| ChryError::Standard(s!(it)))
}


impl State {
    fn entries(&self) -> Vec<Request> {
        let mut result: Vec<Request> = self.processing.iter().map(Clone::clone).collect();
        for it in &self.queued {
            result.push(it.clone());
        }
        result
    }
}


impl Ord for Request {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ticket.cmp(&other.ticket)
    }
}

impl PartialOrd for Request {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.ticket.partial_cmp(&other.ticket)
    }
}

impl Eq for Request {
}

impl PartialEq for Request {
    fn eq(&self, other: &Self) -> bool {
        self.ticket == other.ticket
    }
}
