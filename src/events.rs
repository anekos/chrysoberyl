
use gtk::{Inhibit, main_quit};
use std::sync::mpsc::Sender;

use operation::Operation;
use key::KeyData;




pub fn on_key_press(tx: Sender<Operation>, key: KeyData) -> Inhibit {
    use operation::Operation::*;
    use options::AppOptionName as opt;

    let keyval = key.code;
    if let Some(operation) = match key.text().as_str() {
        "e" => Some(Expand),
        "E" => Some(ExpandRecursive),
        "f" | "h" => Some(First),
        "j" => Some(Next),
        "k" => Some(Previous),
        "l" => Some(Last),
        "q" => Some(Exit),
        "r" => Some(Refresh),
        "i" => Some(Toggle(opt::ShowText)),
        "z" => Some(Shuffle),
        _ => if 48 <= keyval && keyval <= 57 {
            Some(Count((keyval - 48) as u8))
        } else {
            Some(Key(key))
        }
    } {
        tx.send(operation).unwrap();
    }

    Inhibit(false)
}

pub fn on_configure(tx: Sender<Operation>) -> bool {
    tx.send(Operation::Refresh).unwrap();
    false
}

pub fn on_delete() -> Inhibit {
    main_quit();
    Inhibit(false)
}
