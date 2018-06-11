
use std::collections::HashSet;

use glib::Type;
use gtk::prelude::*;
use gtk::{CellRendererText, EditableExt, Entry, EntryBuffer, ListStore, ScrolledWindow, TreeIter, TreePath, TreeSelection, TreeView, TreeViewColumn, Value};

use key::Key;
use completion::definition::{Definition, Argument, Value as Val, OptionValue};
use completion::path::get_candidates;



pub struct CompleterUI {
    pub window: ScrolledWindow,
    candidates: ListStore,
    entry: Entry,
}


#[derive(Debug, PartialEq)]
struct State<'a> {
    args: Vec<&'a str>,
    left: usize,
    right: usize,
    nth: usize,
    text: &'a str,
    debug: &'static str,
}


impl CompleterUI {
    pub fn new(entry: &Entry) -> Self {
        let previous_keys = hashset!{"ISO_Left_Tab", "Up", "C-p"};
        let ignore_keys: HashSet<&'static str> = hashset!{"Tab", "Down", "C-n"}.union(&previous_keys).cloned().collect();

        let definition = Definition::new();
        let candidates = ListStore::new(&[Type::String]);

        let tree_view = tap!(it = TreeView::new_with_model(&candidates), {
            WidgetExt::set_name(&it, "command-line-candidates");

            let cell = CellRendererText::new();

            let column = tap!(it = TreeViewColumn::new(), {
                it.pack_end(&cell, true);
                it.add_attribute(&cell, "text", 0);
                it.set_title("Completion");
            });

            it.set_headers_visible(false);
            it.append_column(&column);
            it.show();

            it.get_selection().connect_changed(|selection| {
                if_let_some!((model, iter) = selection.get_selected(), ());
                if_let_some!(tree_view = selection.get_tree_view(), ());
                let path: Option<TreePath> = model.get_path(&iter);
                tree_view.scroll_to_cell(path.as_ref(), None, false, 0.0, 0.0);
            })
        });

        let window = tap!(it = ScrolledWindow::new(None, None), {
            WidgetExt::set_name(&it, "command-line-completer");
            it.add(&tree_view);
        });

        let entry_buffer = EntryBuffer::new(None);
        entry.set_buffer(&entry_buffer);

        entry.connect_key_press_event(clone_army!([candidates, ignore_keys, previous_keys] move |ref entry, key| {
            let position = entry.get_property_cursor_position();
            let text = entry.get_text().unwrap();
            let state = get_part(&text, position as usize);
            let key = Key::from(key);

            if make_humanism(&key, entry) {
                return Inhibit(true);
            }

            if ignore_keys.contains(key.as_str()) {
                select_next(&tree_view, &candidates, &entry, &entry_buffer, state.left, state.right, previous_keys.contains(key.as_str()));
                return Inhibit(true);
            }

            Inhibit(false)
        }));

        entry.connect_key_release_event(clone_army!([candidates] move |ref entry, key| {
            let position = entry.get_property_cursor_position();
            let text = entry.get_text().unwrap();
            let state = get_part(&text, position as usize);
            let key = Key::from(key);

            if ignore_keys.contains(key.as_str()) {
                return Inhibit(true);
            }

            let result = make_candidates(&state, &definition);
            set_candidates(state.text, &candidates, &result);

            Inhibit(false)
        }));

        CompleterUI { window, candidates, entry: entry.clone() }
    }

    pub fn clear(&self) {
        self.candidates.clear();
        self.entry.set_text("");
    }
}


impl<'a> State<'a> {
    pub fn operation(&self) -> Option<&&str> {
        self.args.first()
    }
}


fn get_part(whole: &str, position: usize) -> State {
    let position = min!(position, whole.len());
    let init_spaces = whole.chars().take_while(|it| *it == ' ').count();

    let mut left = init_spaces;
    let mut right = left;
    let mut after_space = true;
    let mut after_position = false;
    let mut debug = "last";
    let mut nth = 0;
    let mut args = vec![];

    for (i, c) in whole.chars().enumerate().skip(init_spaces) {
        if c == ' ' {
            if after_position {
                break;
            }
            if !after_space {
                nth += 1;
                args.push(&whole[left .. i]);
            }
            left = i;
            right = left;
        } else {
            if after_space && !after_position {
                left = i;
            }
            right = i + 1;
        }

        if position == i + 1 {
            after_position = true;
            debug = "pos";
        }

        after_space = c == ' ';
    }

    State { args, left, right, nth, text: &whole[left .. right], debug }
}

fn make_candidates(state: &State, definition: &Definition) -> Vec<String> {
    fn get_flag_name(s: &str) -> Option<&str> {
        if s.starts_with("--") {
            Some(&s[2..])
        } else if s.starts_with('-') {
            Some(&s[1..])
        } else {
            None
        }
    }

    let make = |value: &Val, option_name: Option<&&str>, result: &mut Vec<String>| {
        match *value {
            Val::OptionName =>
                result.extend_from_slice(&*definition.options),
            Val::Literals(ref values) =>
                result.extend_from_slice(&*values),
            Val::Directory =>
                get_candidates(&state.text, true, "", result),
            Val::File | Val::Path =>
                get_candidates(&state.text, false, "", result),
            Val::OptionValue => {
                if_let_some!(option_name = option_name, ());
                if let Some(value) = definition.option_values.get(*option_name) {
                    match value {
                        OptionValue::Enum(values) => result.extend_from_slice(&*values),
                        OptionValue::Boolean => result.extend_from_slice(&[o!("true"), o!("false")]),
                        OptionValue::StringOrFile => get_candidates(&state.text, false, "@", result),
                    }
                }
            }
            _ => (),
        }
    };

    if_let_some!(operation = state.operation(), definition.operations.clone());
    let mut result = vec![];
    if_let_some!(def_args = definition.arguments.get(&operation[1..]), result);

    let mut skip = false;
    let mut arg_nth = 0;

    for (i, arg) in state.args.iter().skip(1).enumerate() {
        if skip {
            skip = false;
            continue;
        }

        if let Some(flag_name) = get_flag_name(arg) {
            if let Some(Argument::Flag(_, flag_value)) = def_args.iter().find(|it| {
                if let Argument::Flag(names, _) = it {
                    names.contains(&o!(flag_name))
                } else {
                    false
                }
            }) {
                if let Some(flag_value) = flag_value {
                    if i == state.args.len() - 1 { // at last
                        make(&flag_value, None, &mut result);
                        return result;
                    } else {
                        skip = true;
                    }
                }
            };
        } else {
            arg_nth += 1;
        }
    }

    {
        let arg_nth = arg_nth;
        let mut n = 0;
        for arg in def_args.iter() {
            match arg {
                Argument::Arg(ref value) => {
                    if arg_nth == n {
                        make(value, state.args.get(1), &mut result);
                    }
                    n += 1;
                }
                Argument::Flag(names, _) if arg_nth == 0 =>
                    result.push(format!("--{}", names[0])),
                _ => (),
            }
        }
    }

    result
}

fn make_humanism(key: &Key, entry: &Entry) -> bool {
    use gtk::{DeleteType, MovementStep};

    match key.as_str() {
        "C-A"=> {
            entry.emit_move_cursor(MovementStep::BufferEnds, 1, false);
            entry.emit_move_cursor(MovementStep::BufferEnds, -1, true);
        },
        "C-h" => entry.emit_delete_from_cursor(DeleteType::Chars, -1),
        "C-k" => entry.emit_delete_from_cursor(DeleteType::DisplayLineEnds, 1),
        "C-u" => entry.emit_delete_from_cursor(DeleteType::DisplayLines, -1),
        "C-w" => entry.emit_delete_from_cursor(DeleteType::WordEnds, -1),
        "C-e" => entry.emit_move_cursor(MovementStep::BufferEnds, 1, false),
        "C-a" => entry.emit_move_cursor(MovementStep::BufferEnds, -1, false),
        "C-f" => entry.emit_move_cursor(MovementStep::LogicalPositions, 1, false),
        "C-b" => entry.emit_move_cursor(MovementStep::LogicalPositions, -1, false),
        _ => return false,
    }

    true
}

fn next_iter(model: &ListStore, selection: &TreeSelection, reverse: bool) -> Option<TreeIter> {
    if reverse {
        if let Some((_, iter)) = selection.get_selected() {
            if model.iter_previous(&iter) {
                return Some(iter)
            }
        }
        let n = model.iter_n_children(None);
        let path = TreePath::new_from_indicesv(&[n - 1, -1]);
        model.get_iter(&path)
    } else {
        if let Some((_, iter)) = selection.get_selected() {
            if model.iter_next(&iter) {
                return Some(iter)
            }
        }
        model.get_iter_first()
    }
}

fn select_next(tree_view: &TreeView, model: &ListStore, entry: &Entry, entry_buffer: &EntryBuffer, left: usize, right: usize, reverse: bool) {
    let len = right - left;
    let selection = tree_view.get_selection();
    if_let_some!(iter = next_iter(model, &selection, reverse), ());
    selection.select_iter(&iter);

    if_let_some!(value = model.get_value(&iter, 0).get::<String>(), ());

    entry_buffer.delete_text(left as u16, Some(len as u16));
    entry_buffer.insert_text(left as u16 + 1, &value);
    entry.set_position((left + value.len() + 1) as i32);
}

fn set_candidates(mut part: &str, store: &ListStore, candidates: &[String]) {
    if part.starts_with('@') {
        part = &part[1..];
    }
    store.clear();
    for candidate in candidates {
        set_if_match(store, part, candidate);
    }
}

fn set_if_match(store: &ListStore, part: &str, candidate: &str) {
    if candidate.contains(part) {
        let iter = store.append();
        let value = Value::from(candidate);
        store.set_value(&iter, 0, &value);
    }
}




#[cfg(test)]#[test]
fn test_get_part() {
    assert_eq!(
        get_part("", 0),
        State {
            args: vec![],
            left: 0,
            right: 0,
            nth: 0,
            text: "",
            debug: "last"
        });
    assert_eq!(
        get_part("@", 1),
        State {
            args: vec![],
            left: 0,
            right: 1,
            nth: 0,
            text: "@",
            debug: "pos"
        });
    assert_eq!(
        get_part("@p", 2),
        State {
            args: vec![],
            left: 0,
            right: 2,
            nth: 0,
            text: "@p",
            debug: "pos"
        });
    assert_eq!(
        get_part("@prev", 5),
        State {
            args: vec![],
            left: 0,
            right: 5,
            nth: 0,
            text: "@prev",
            debug: "pos"
        });
    assert_eq!(
        get_part("@prev ", 6),
        State {
            args: vec!["@prev"],
            left: 5,
            right: 5,
            nth: 1,
            text: "",
            debug: "pos"
        });
    assert_eq!(
        get_part("@prev arg", 9),
        State {
            args: vec!["@prev"],
            left: 6,
            right: 9,
            nth: 1,
            text: "arg",
            debug: "pos"
        });

    assert_eq!(
        get_part("@p", 1),
        State {
            args: vec![],
            left: 0,
            right: 2,
            nth: 0,
            text: "@p",
            debug: "pos"
        });

    assert_eq!(
        get_part("@foo -i -v", 10),
        State {
            args: vec!["@foo", "-i"],
            left: 8,
            right: 10,
            nth: 2,
            text: "-v",
            debug: "pos"
        });
}
