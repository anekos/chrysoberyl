
use gtk::prelude::*;
use glib::Type;
use gtk::{CellRendererText, Entry, EntryBuffer, ListStore, ScrolledWindow, TreeIter, TreeSelection, TreeView, TreeViewColumn, Value, EditableExt};

use completion::definition::Definition;



pub struct CompleterUI {
    pub window: ScrolledWindow,
}


impl CompleterUI {
    pub fn new(entry: &Entry) -> Self {
        let definition = Definition::new();

        let candidates = ListStore::new(&[Type::String]);

        let tree_view = tap!(it = TreeView::new_with_model(&candidates), {
            let cell = CellRendererText::new();
            let column = TreeViewColumn::new();
            column.pack_end(&cell, true);
            column.add_attribute(&cell, "text", 0);
            column.set_title("Completion");
            it.append_column(&column);
            it.show();
        });

        let window = tap!(it = ScrolledWindow::new(None, None), {
            it.add(&tree_view);
        });

        let entry_buffer = EntryBuffer::new(None);
        entry.set_buffer(&entry_buffer);

        entry.connect_key_release_event(move |ref entry, key| {
            use ::gdk::enums::key::*;

            let position = entry.get_property_cursor_position();
            let text = entry.get_text().unwrap();

            if let Some((part, left, len)) = get_part(&text, position as usize) {
                let key = key.as_ref().keyval;

                if key == Tab {
                    select_next(&tree_view, &candidates, &entry, &entry_buffer, left, len);
                    return Inhibit(true);
                }

                set_operations(part, &candidates, &definition.operations);
            } else {
                candidates.clear();
            }

            Inhibit(false)
        });

        CompleterUI { window }
    }
}


fn get_part(whole: &str, position: usize) -> Option<(&str, usize, usize)> {
    let position = min!(position, whole.len());
    let mut in_part = false;
    let mut left = 0;
    let len = whole.len();

    for (i, c) in whole.chars().enumerate() {
        let i = i + 1;

        if position == i {
            if c == ' ' {
                return None;
            }
            in_part = true;
        }

        if c == ' ' {
            if in_part {
                return Some((&whole[left .. i - 1], left, len - left));
            }
            left = i;
        }
    }

    if in_part {
        Some((&whole[left .. len], left, len - left))
    } else {
        None
    }
}


fn set_if_match(store: &ListStore, part: &str, candidate: &str) {
    if candidate.starts_with(part) {
        let iter = store.append();
        let value = Value::from(candidate);
        store.set_value(&iter, 0, &value);
    }
}

fn set_operations(part: &str, store: &ListStore, candidates: &[String]) {
    store.clear();
    for candidate in candidates {
        set_if_match(store, part, candidate);
    }
}

fn next_iter(model: &ListStore, selection: &TreeSelection) -> Option<TreeIter> {
    if let Some((_, iter)) = selection.get_selected() {
        if model.iter_next(&iter) {
            return Some(iter)
        }
    }
    model.get_iter_first()
}

fn select_next(tree_view: &TreeView, model: &ListStore, entry: &Entry, entry_buffer: &EntryBuffer, left: usize, len: usize) {
    let selection = tree_view.get_selection();
    if_let_some!(iter = next_iter(model, &selection), ());
    selection.select_iter(&iter);

    if_let_some!(value = model.get_value(&iter, 0).get::<String>(), ());

    entry_buffer.delete_text(left as u16, Some(len as u16));
    entry_buffer.insert_text(left as u16, &value);
    entry.set_position((left + value.len()) as i32);
}
