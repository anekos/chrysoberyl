
use gtk::prelude::*;
use glib::Type;
use gtk::{CellRendererText, Entry, EntryBuffer, ListStore, ScrolledWindow, TreeIter, TreeSelection, TreeView, TreeViewColumn, Value, EditableExt};

use completion::definition::{Definition, Argument, Value as Val, OptionValue};
use completion::path::get_candidates;



pub struct CompleterUI {
    pub window: ScrolledWindow,
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

            let result = make_candidates(&state, &definition);
            set_candidates(state.text, &candidates, &result);

            Inhibit(false)
        });

        CompleterUI { window }
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
                get_candidates(&state.text, true, result),
            Val::File | Val::Path =>
                get_candidates(&state.text, false, result),
            Val::OptionValue => {
                if_let_some!(option_name = option_name, ());
                if let Some(value) = definition.option_values.get(*option_name) {
                    match value {
                        OptionValue::Enum(values) => result.extend_from_slice(&*values),
                        OptionValue::Boolean => result.extend_from_slice(&[o!("true"), o!("false")]),
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
