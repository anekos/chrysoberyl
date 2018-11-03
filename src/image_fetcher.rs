
use std::collections::VecDeque;
use std::default::Default;
use std::sync::Arc;
use std::sync::mpsc::{channel, Sender};
use std::thread::spawn;

use num_cpus;

use entry::image::Imaging;
use entry::{Entry, Key, self};
use image::ImageBuffer;
use image_cache::ImageCache;
use size::Size;
use state::Drawing;



pub struct ImageFetcher {
    main_tx: Sender<FetcherOperation>,
}

pub struct FetchTarget {
    cell_size: Size,
    drawing: Drawing,
    entries: VecDeque<Arc<Entry>>,
}

pub enum FetcherOperation {
    Refresh(FetchTarget),
    Done(Key, Result<ImageBuffer, String>),
}


impl ImageFetcher {
    pub fn new(image_cache: ImageCache) -> ImageFetcher {
        ImageFetcher {
            main_tx: main(image_cache)
        }
    }

    pub fn new_target(&self, entries: VecDeque<Arc<Entry>>, cell_size: Size, drawing: Drawing) {
        self.main_tx.send(
            FetcherOperation::Refresh(
                FetchTarget {
                    cell_size,
                    drawing,
                    entries,
                })).unwrap();
    }
}


impl FetchTarget {
    fn get_imaging(&self) -> Imaging {
        Imaging::new(self.cell_size, self.drawing.clone())
    }
}

impl Default for FetchTarget {
    fn default() -> FetchTarget {
        FetchTarget {
            cell_size: Size::new(0, 0),
            drawing: Drawing::default(),
            entries: VecDeque::new()
        }
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
                    let imaging = current_target.get_imaging();
                    start(
                        &tx,
                        &mut cache,
                        &mut current_target.entries,
                        &mut idles,
                        &imaging);
                }
                Done(key, image_buffer) => {
                    idles += 1;
                    let imaging = current_target.get_imaging();
                    cache.push(&imaging, &key, image_buffer);
                    start(&tx, &mut cache, &mut current_target.entries, &mut idles, &imaging);
                }
            }
        }
    }));

    tx
}


pub fn start(tx: &Sender<FetcherOperation>, cache: &mut ImageCache, entries: &mut VecDeque<Arc<Entry>>, idles: &mut usize, imaging: &Imaging) {
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


pub fn fetch(tx: Sender<FetcherOperation>, entry: Arc<Entry>, imaging: Imaging) {
    spawn(move || {
        let image = entry::image::get_image_buffer(&entry, &imaging).map_err(|it| s!(it));
        tx.send(FetcherOperation::Done(entry.key.clone(), image)).unwrap();
    });
}
