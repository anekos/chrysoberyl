
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::Duration;

use notify::{self, Watcher as W, DebouncedEvent, RecursiveMode};
use operation::Operation;



pub struct Watcher {
    tx: Sender<WatcherCommand>,
}

#[derive(Debug)]
enum WatcherCommand {
    Update(HashSet<PathBuf>),
    Notified(PathBuf),
}


impl Watcher {
    pub fn new(app_tx: Sender<Operation>) -> Self {
        let tx = main(app_tx);

        Watcher { tx }
    }

    pub fn update(&self, targets: HashSet<PathBuf>) {
        self.tx.send(WatcherCommand::Update(targets)).unwrap();
    }
}


fn main(app_tx: Sender<Operation>) -> Sender<WatcherCommand> {
    let (tx, rx) = channel();

    thread::spawn(clone_army!([tx] move || {
        let (ntx, nrx) = channel();
        if_let_ok!(mut w = notify::watcher(ntx, Duration::from_millis(100)), |err| puts_error!(err));
        let mut watchings = HashSet::<PathBuf>::new();

        n_main(nrx, tx.clone());

        while let Ok(command) = rx.recv() {
            match command {
                WatcherCommand::Notified(path) =>
                    ignore!(app_tx.send(Operation::FileChanged(path))),
                WatcherCommand::Update(new_targets) => {
                    for it in &watchings {
                        ignore!(w.unwatch(it));
                    }
                    for it in &new_targets {
                        ignore!(w.watch(it, RecursiveMode::NonRecursive));
                    }
                    watchings = new_targets;
                }
            }
        }
    }));

    tx
}

fn n_main(nrx: Receiver<DebouncedEvent>, tx: Sender<WatcherCommand>) {
    use self::DebouncedEvent::*;

    thread::spawn(move || {
        while let Ok(event) = nrx.recv() {
            match event {
                Create(path) | Write(path) | Rename(_, path) =>
                    ignore!(tx.send(WatcherCommand::Notified(path))),
                _ => (),
            }
        }
    });
}
