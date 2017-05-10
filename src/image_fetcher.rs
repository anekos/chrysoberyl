
use std::collections::VecDeque;
use std::default::Default;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::spawn;

use num_cpus;

use entry::Entry;
use entry_image::get_image_buffer;
use image_cache::ImageCache;
use size::Size;
use state::DrawingOption;


type ArcTarget = Arc<Mutex<FetchTarget>>;

pub struct ImageFetcher {
    main_tx: Sender<FetcherOperation>,
}

pub struct FetchTarget {
    cell_size: Size,
    drawing: DrawingOption,
    entries: VecDeque<Entry>,
}

pub enum FetcherOperation {
    Refresh(FetchTarget),
    Done
}


impl ImageFetcher {
    pub fn new(image_cache: ImageCache) -> ImageFetcher {
        let target = Arc::new(Mutex::new(FetchTarget::default()));
        ImageFetcher {
            main_tx: main(target, image_cache)
        }
    }

    pub fn new_target(&self, entries: VecDeque<Entry>, cell_size: Size, drawing: DrawingOption) {
        self.main_tx.send(
            FetcherOperation::Refresh(
                FetchTarget {
                    cell_size: cell_size,
                    drawing: drawing,
                    entries: entries
                })).unwrap();
    }
}


impl Default for FetchTarget {
    fn default() -> FetchTarget {
        FetchTarget {
            cell_size: Size::new(0, 0),
            drawing: DrawingOption::default(),
            entries: VecDeque::new()
        }
    }
}


fn main(target: ArcTarget, cache: ImageCache) -> Sender<FetcherOperation> {
    use self::FetcherOperation::*;

    let (tx, rx) = channel();

    spawn(clone_army!([tx] move || {
        let mut idles = num_cpus::get();

        info!("image_fetcher: threads={}", idles);

        while let Ok(op) = rx.recv() {
            match op {
                Refresh(new_targets) => {
                    let mut target = target.lock().unwrap();
                    *target = new_targets;
                    println!("refresh: {:?}", idles);
                    start(&tx, cache.clone(), &mut idles, &mut target);
                }
                Done => {
                    let mut target = target.lock().unwrap();
                    idles += 1;
                    start(&tx, cache.clone(), &mut idles, &mut target);
                }
            }
        }
    }));

    tx
}


pub fn start(tx: &Sender<FetcherOperation>, mut cache: ImageCache, idles: &mut usize, target: &mut FetchTarget) {
    for _ in 0..*idles {
        if let Some(entry) = target.entries.pop_front() {
            if cache.fetching(entry.key.clone()) {
                *idles -= 1;
                fetch(tx.clone(), cache.clone(), entry, target.cell_size, target.drawing.clone());
            }
        }
    }
}


pub fn fetch(tx: Sender<FetcherOperation>, mut cache: ImageCache, entry: Entry, cell_size: Size, drawing: DrawingOption) {
    cache.push(entry, move |entry| {
        let result = get_image_buffer(&entry, &cell_size, &drawing);
        tx.send(FetcherOperation::Done).unwrap();
        result
    });
}
