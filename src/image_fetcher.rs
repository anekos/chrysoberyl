
use std::collections::VecDeque;
use std::default::Default;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;

use closet::clone_army;
use log::{info, trace};

use crate::entry::image::Imaging;
use crate::entry::{Entry, Key, self};
use crate::image::ImageBuffer;
use crate::image_cache::ImageCache;



pub struct ImageFetcher {
    main_tx: Sender<FetcherOperation>,
}

#[derive(Default)]
pub struct FetchTarget {
    imaging: Imaging,
    entries: VecDeque<Arc<Entry>>,
}

pub enum FetcherOperation {
    Refresh(FetchTarget),
    Done(Key, Imaging, Result<ImageBuffer, String>),
}


impl ImageFetcher {
    pub fn new(image_cache: ImageCache) -> ImageFetcher {
        ImageFetcher {
            main_tx: main(image_cache)
        }
    }

    pub fn new_target(&self, entries: VecDeque<Arc<Entry>>, imaging: Imaging) {
        self.main_tx.send(
            FetcherOperation::Refresh(
                FetchTarget {
                    imaging,
                    entries,
                })).unwrap();
    }
}



fn main(mut cache: ImageCache) -> Sender<FetcherOperation> {
    use self::FetcherOperation::*;

    let (tx, rx) = channel();

    spawn(clone_army!([tx] move || {
        let mut idles = num_cpus::get();
        let mut current_target = FetchTarget::default();

        info!("image_fetcher: threads={}", idles);

        while let Ok(op) = rx.recv() {
            match op {
                Refresh(new_targets) => {
                    current_target = new_targets;
                    start(
                        &tx,
                        &mut cache,
                        &mut current_target.entries,
                        &mut idles,
                        &current_target.imaging);
                }
                Done(key, imaging, image_buffer) => {
                    idles += 1;
                    cache.push(&imaging, &key, image_buffer);
                    start(&tx, &mut cache, &mut current_target.entries, &mut idles, &current_target.imaging);
                }
            }
        }
    }));

    tx
}


fn start(tx: &Sender<FetcherOperation>, cache: &mut ImageCache, entries: &mut VecDeque<Arc<Entry>>, idles: &mut usize, imaging: &Imaging) {
    while 0 < *idles {
        if let Some(entry) = entries.pop_front() {
            if cache.mark_fetching(imaging, entry.key.clone()) {
                *idles -= 1;
                fetch(tx.clone(), entry, imaging.clone());
            }
        } else {
            return;
        }
    }
}


fn fetch(tx: Sender<FetcherOperation>, entry: Arc<Entry>, imaging: Imaging) {
    spawn(move || {
        trace!("image_fetcher/get_image_buffer: key={:?}", &(*entry).key);
        let image_buffer = entry::image::get_image_buffer(&(*entry).content, &imaging).map_err(|it| s!(it));
        tx.send(FetcherOperation::Done(entry.key.clone(), imaging, image_buffer)).unwrap();
    });
}
