
use std::collections::HashMap;
use std::hash::Hash;
use std::mem::swap;
use std::rc::Rc;
use std::slice;

use rand::{thread_rng, Rng, ThreadRng};



pub type Pred<T> = Box<FnMut(&mut T) -> bool>;

pub struct FilterableVec<T: Clone + Hash + Eq + Sized> {
    original: Vec<Rc<T>>,
    filtered: Vec<Rc<T>>,
    original_indices: HashMap<Rc<T>, usize>,
    filtered_indices: HashMap<Rc<T>, usize>,
    rng: ThreadRng,
    dynamic_pred: Option<Pred<T>>,
    static_pred: Option<Pred<T>>,
}


impl<T: Clone + Hash + Eq + Sized + Ord> FilterableVec<T> {
    pub fn new() -> Self {
        FilterableVec {
            original: vec![],
            filtered: vec![],
            original_indices: HashMap::new(),
            filtered_indices: HashMap::new(),
            rng: thread_rng(),
            dynamic_pred: None,
            static_pred: None,
        }
    }

    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    pub fn contains(&self, entry: &T) -> bool {
        self.get_index(entry).is_some()
    }

    pub fn get_index(&self, entry: &T) -> Option<usize> {
        self.filtered_indices.get(entry).cloned()
    }

    pub fn iter(&self) -> slice::Iter<Rc<T>> {
        self.filtered.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Rc<T>> {
        self.filtered.get(index)
    }

    pub fn validate_nth(&mut self, index: usize, mut pred: Pred<T>) -> Option<bool> {
        self.filtered.get_mut(index).map(|mut it| {
            (*pred)(Rc::make_mut(&mut it))
        })
    }

    pub fn split_at(&self, index: usize) -> (&[Rc<T>], &[Rc<T>]) {
        self.filtered.split_at(index)
    }

    pub fn clone_filtered(&self) -> Vec<Rc<T>> {
        self.filtered.clone()
    }

    pub fn clear(&mut self) {
        self.original.clear();
        self.filtered.clear();
        self.filtered_indices.clear();
        self.original_indices.clear();
    }

    pub fn sort(&mut self, index_before_filter: Option<usize>) -> Option<usize> {
        self.original.sort();
        self.filter(index_before_filter)
    }

    // Shuffle **original** entries
    pub fn shuffle(&mut self) {
        let mut source = self.original.clone();
        let mut buffer = source.as_mut_slice();
        self.rng.shuffle(&mut buffer);
        self.original = buffer.to_vec();

        // FIXME Optimize
        self.filter(None);
    }

    pub fn extend_from_slice(&mut self, entries: &[Rc<T>]) {
        let len = self.original.len();

        let entries =
            if let Some(ref mut static_pred) = self.static_pred {
                let entries = entries.to_vec();
                let mut targets = vec![];
                for mut entry in &mut entries.into_iter() {
                    if (static_pred)(Rc::make_mut(&mut entry)) {
                        targets.push(entry.clone());
                    }
                }
                targets
            } else {
                entries.to_vec()
            };

        self.original.extend_from_slice(&*entries);

        let targets = if let Some(ref mut dynamic_pred) = self.dynamic_pred {
            let mut targets = vec![];
            for (index, mut entry) in &mut entries.into_iter().enumerate() {
                if (dynamic_pred)(Rc::make_mut(&mut entry)) {
                    self.original_indices.insert(entry.clone(), len + index);
                    targets.push(entry.clone());
                }
            }
            targets
        } else {
            self.filtered.extend_from_slice(&*entries);
            self.reset_indices(); // FXIME Optimize
            return;
        };

        for target in targets {
            self.push_filtered(target);
        }
    }

    pub fn push(&mut self, mut entry: Rc<T>) {
        if let Some(ref mut static_pred) = self.static_pred {
            if !(static_pred)(Rc::make_mut(&mut entry)) {
                return;
            }
        };

        self.original_indices.insert(entry.clone(), self.original.len());
        self.original.push(entry.clone());

        if let Some(ref mut dynamic_pred) = self.dynamic_pred {
            if !(dynamic_pred)(Rc::make_mut(&mut entry)) {
                return;
            }
        };

        self.push_filtered(entry.clone());
    }

    pub fn update_filter(&mut self, dynamic: bool, index_before_filter: Option<usize>, pred: Option<Pred<T>>) -> Option<usize> {
        if dynamic {
            self.dynamic_pred = pred;
        } else {
            self.static_pred = pred;
        }
        self.filter(index_before_filter)
    }

    pub fn delete(&mut self, index_before_filter: Option<usize>, pred: Pred<T>) -> Option<usize> {
        let mut pred = Some(pred);
        swap(&mut pred, &mut self.static_pred);
        let result = self.filter(index_before_filter);
        swap(&mut pred, &mut self.static_pred);
        result
    }

    pub fn filter(&mut self, index_before_filter: Option<usize>) -> Option<usize> {
        if let Some(ref mut static_pred) = self.static_pred {
            let mut new_originals = vec![];
            for mut entry in &mut self.original.iter_mut() {
                if (static_pred)(Rc::make_mut(&mut entry)) {
                    new_originals.push(entry.clone());
                }
            }
            self.original = new_originals;
        }

        let original_index_before: Option<usize> = index_before_filter.and_then(|bi| {
            self.filtered.get(bi).and_then(|entry| {
                self.original_indices.get(entry).cloned()
            })
        });

        let (mut after_index_left, mut after_index_right) = (None, None);

        if let Some(ref mut dynamic_pred) = self.dynamic_pred {
            self.filtered = vec![];
            for (index, mut entry) in &mut self.original.iter_mut().enumerate() {
                if (dynamic_pred)(Rc::make_mut(&mut entry)) {
                     self.filtered.push(entry.clone());

                     if let Some(original_index_before) = original_index_before {
                         if index == original_index_before {
                             after_index_right = Some(index);
                         } else if index < original_index_before {
                             after_index_left = Some(index);
                         } else if after_index_right.is_none() {
                             if let Some(after_index_left) = after_index_left {
                                 if index - original_index_before < original_index_before - after_index_left {
                                     after_index_right = Some(index);
                                 }
                             } else {
                                 after_index_right = Some(index);
                             }
                         }
                     }
                }
            }
        } else {
            self.filtered = self.original.clone();
        }
        self.reset_indices();

        after_index_right.or(after_index_left).map(|index| {
            self.filtered_indices[&self.original[index]]
        })
    }

    fn push_filtered(&mut self, entry: Rc<T>) {
        self.filtered_indices.insert(entry.clone(), self.filtered.len());
        self.filtered.push(entry);
    }

    fn reset_indices(&mut self) {
        self.filtered_indices.clear();
        for (index, entry) in self.filtered.iter().enumerate() {
            self.filtered_indices.insert(entry.clone(), index);
        }
        self.original_indices.clear();
        for (index, entry) in self.original.iter().enumerate() {
            self.original_indices.insert(entry.clone(), index);
        }
    }
}
