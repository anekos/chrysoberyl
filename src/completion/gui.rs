
use gtk::prelude::*;
use glib::Type;
use gtk::{CellRendererText, Entry, EntryBuffer, ListStore, ScrolledWindow, TreeIter, TreeSelection, TreeView, TreeViewColumn, Value, EditableExt};

use completion::definition::{Definition, Argument};



pub struct CompleterUI {
    pub window: ScrolledWindow,
}


#[derive(Debug, PartialEq)]
struct State<'a> {
    left: usize,
    right: usize,
    num: usize,
    operation: Option<&'a str>,
    text: &'a str,
    debug: &'static str,
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
            let state = get_part(&text, position as usize);
            let key = key.as_ref().keyval;

            if key == Tab {
                select_next(&tree_view, &candidates, &entry, &entry_buffer, state.left, state.right);
                return Inhibit(true);
            }

            if let Some(operation) = state.operation {
                if let Some(args) = definition.arguments.get(&operation[1..]) {
                    let mut cs = vec![];
                    for arg in args.iter() {
                        if let Argument::Flag(names, _) = arg {
                            cs.push(format!("--{}", names[0]));
                        }
                    }
                    set_candidates(state.text, &candidates, &cs);
                } else {
                    candidates.clear();
                }
            } else {
                set_candidates(state.text, &candidates, &definition.operations);
            }

            Inhibit(false)
        });

        CompleterUI { window }
    }
}


fn get_part(whole: &str, position: usize) -> State {
    let position = min!(position, whole.len());
    let init_spaces = whole.chars().take_while(|it| *it == ' ').count();

    let mut operation = None;
    let mut left = init_spaces;
    let mut right = left;
    let mut after_space = true;
    let mut debug = "last";

    for (i, c) in whole.chars().enumerate().skip(init_spaces) {
        if c == ' ' {
            if operation.is_none() {
                operation = Some(&whole[init_spaces .. i])
            }
            left = i;
            right = left;
        } else {
            if after_space {
                left = i;
            }
            right = i + 1;
        }

        if position == i + 1 {
            debug = "pos";
            break;
        }

        after_space = c == ' ';
    }

    State { left, right, num: 0, operation, text: &whole[left .. right], debug }
}


fn set_if_match(store: &ListStore, part: &str, candidate: &str) {
    if candidate.starts_with(part) {
        let iter = store.append();
        let value = Value::from(candidate);
        store.set_value(&iter, 0, &value);
    }
}

fn set_candidates(part: &str, store: &ListStore, candidates: &[String]) {
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

fn select_next(tree_view: &TreeView, model: &ListStore, entry: &Entry, entry_buffer: &EntryBuffer, left: usize, right: usize) {
    let len = right - left;
    let selection = tree_view.get_selection();
    if_let_some!(iter = next_iter(model, &selection), ());
    selection.select_iter(&iter);

    if_let_some!(value = model.get_value(&iter, 0).get::<String>(), ());

    entry_buffer.delete_text(left as u16, Some(len as u16));
    entry_buffer.insert_text(left as u16 + 1, &value);
    entry.set_position((left + value.len() + 1) as i32);
}



#[cfg(test)]#[test]
fn test_get_part() {
    assert_eq!(get_part("", 0), State { left: 0,  right: 0, num: 0, operation: None, text: "", debug: "last" });
    assert_eq!(get_part("@", 1), State { left: 0,  right: 1, num: 0, operation: None, text: "@", debug: "pos" });
    // assert_eq!(get_part("@p", 1), State { left: 0,  right: 2, num: 0, operation: None, text: "@p", debug: "pos" });
    assert_eq!(get_part("@p", 2), State { left: 0,  right: 2, num: 0, operation: None, text: "@p", debug: "pos" });
    assert_eq!(get_part("@prev", 5), State { left: 0,  right: 5, num: 0, operation: None, text: "@prev", debug: "pos" });
    assert_eq!(get_part("@prev ", 6), State { left: 5,  right: 5, num: 0, operation: Some("@prev"), text: "", debug: "pos" });
    assert_eq!(get_part("@prev arg", 9), State { left: 6,  right: 9, num: 0, operation: Some("@prev"), text: "arg", debug: "pos" });
}
