
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::mem::swap;
use std::slice;
use std::sync::Arc;

use rand::{thread_rng, Rng, ThreadRng};



pub type Pred<T, U> = Box<Fn(&T, &U) -> bool>;

pub struct FilterableVec<T: Hash + Eq + Sized, U> {
    original: Vec<Arc<T>>,
    filtered: Vec<Arc<T>>,
    original_indices: HashMap<Arc<T>, usize>,
    filtered_indices: HashMap<Arc<T>, usize>,
    rng: ThreadRng,
    dynamic_pred: Option<Pred<T, U>>,
    static_pred: Option<Pred<T, U>>,
}


impl<T: Hash + Eq + Sized + Ord, U> FilterableVec<T, U> {
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

    pub fn real_len(&self) -> usize {
        self.original.len()
    }

    pub fn contains(&self, entry: &T) -> bool {
        self.get_index(entry).is_some()
    }

    pub fn get_index(&self, entry: &T) -> Option<usize> {
        self.filtered_indices.get(entry).cloned()
    }

    pub fn iter(&self) -> slice::Iter<Arc<T>> {
        self.filtered.iter()
    }

    pub fn get(&self, index: usize) -> Option<Arc<T>> {
        self.filtered.get(index).cloned()
    }

    pub fn validate_nth(&mut self, index: usize, info: &U, pred: &Pred<T, U>) -> Option<bool> {
        self.filtered.get(index).map(|it| (*pred)(it, info))
    }

    pub fn split_at(&self, index: usize) -> (&[Arc<T>], &[Arc<T>]) {
        self.filtered.split_at(index)
    }

    pub fn clone_filtered(&self) -> Vec<Arc<T>> {
        // fixme
        // self.filtered.iter().map(|it| it.clone()).collect()
        vec![]
    }

    pub fn clear(&mut self) {
        self.original.clear();
        self.filtered.clear();
        self.filtered_indices.clear();
        self.original_indices.clear();
    }

    pub fn sort(&mut self, info: &U) -> Option<usize> {
        self.original.sort();
        self.filter(info, None)
    }

    pub fn sort_by<F>(&mut self, info: &U, compare: F) -> Option<usize> where F: Fn(&T, &T) -> Ordering {
        {
            let len = self.original.len();
            let xs: &mut [Arc<T>] = self.original.as_mut_slice();
            quicksort::<T, F>(xs, 0, len, &compare);
        }
        self.filter(info, None)
    }

    // Shuffle **original** entries
    pub fn shuffle(&mut self, info: &U) {
        let mut source = self.original.clone();
        let mut buffer = source.as_mut_slice();
        self.rng.shuffle(&mut buffer);
        self.original = buffer.to_vec();

        // FIXME Optimize
        self.filter(info, None);
    }

    pub fn extend_from_slice(&mut self, info: &U, entries: &[Arc<T>]) {
        let len = self.original.len();

        let entries =
            if let Some(ref static_pred) = self.static_pred {
                let entries = entries.to_vec();
                let mut targets = vec![];
                for entry in entries {
                    if static_pred(&entry, info) {
                        targets.push(Arc::clone(&entry));
                    }
                }
                targets
            } else {
                entries.to_vec()
            };

        self.original.extend_from_slice(&*entries);

        let targets = if let Some(ref dynamic_pred) = self.dynamic_pred {
            let mut targets = vec![];
            for (index, entry) in entries.iter().enumerate() {
                if dynamic_pred(entry, info) {
                    self.original_indices.insert(Arc::clone(entry), len + index);
                    targets.push(Arc::clone(entry));
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

    pub fn push(&mut self, info: &U, entry: &Arc<T>) {
        if let Some(ref static_pred) = self.static_pred {
            if !static_pred(&*entry, info) {
                return;
            }
        };

        self.original_indices.insert(Arc::clone(entry), self.original.len());
        self.original.push(Arc::clone(entry));

        if let Some(ref dynamic_pred) = self.dynamic_pred {
            if !(dynamic_pred)(&*entry, info) {
                return;
            }
        };

        self.push_filtered(Arc::clone(entry));
    }

    pub fn remove(&mut self, indices: &HashSet<usize>, info: &U) {
        let mut old = vec![];
        swap(&mut old, &mut self.original);

        for (index, entry) in old.into_iter().enumerate() {
            if !indices.contains(&index) {
                self.original.push(entry);
            }
        }

        self.filter(info, None);
    }

    pub fn update_filter(&mut self, info: &U, dynamic: bool, index_before_filter: Option<usize>, pred: Option<Pred<T, U>>) -> Option<usize> {
        if dynamic {
            self.dynamic_pred = pred;
        } else {
            self.static_pred = pred;
        }
        self.filter(info, index_before_filter)
    }

    pub fn delete(&mut self, info: &U, index_before_filter: Option<usize>, pred: Pred<T, U>) -> Option<usize> {
        let mut pred = Some(pred);
        swap(&mut pred, &mut self.static_pred);
        let result = self.filter(info, index_before_filter);
        swap(&mut pred, &mut self.static_pred);
        result
    }

    pub fn filter(&mut self, info: &U, index_before_filter: Option<usize>) -> Option<usize> {
        if let Some(ref mut static_pred) = self.static_pred {
            let mut new_originals = vec![];
            for mut entry in &mut self.original.iter_mut() {
                if (static_pred)(entry, info) {
                    new_originals.push(Arc::clone(entry));
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
                if dynamic_pred(entry, info) {
                     self.filtered.push(Arc::clone(entry));

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

    fn push_filtered(&mut self, entry: Arc<T>) {
        self.filtered_indices.insert(Arc::clone(&entry), self.filtered.len());
        self.filtered.push(entry);
    }

    fn reset_indices(&mut self) {
        self.filtered_indices.clear();
        for (index, entry) in self.filtered.iter().enumerate() {
            self.filtered_indices.insert(Arc::clone(entry), index);
        }
        self.original_indices.clear();
        for (index, entry) in self.original.iter().enumerate() {
            self.original_indices.insert(Arc::clone(entry), index);
        }
    }
}


fn partition<T, F>(xs: &mut [Arc<T>], left: usize, right: usize, compare: &F) -> usize where F: Fn(&T, &T) -> Ordering {
    let mut i = 0;

    {
        let (lefts, rights) = xs.split_at_mut(left + 1);
        let pivot: &mut Arc<T> = &mut lefts[left];
        for j in 0..(right - left - 1) {
            let less = {
                let it: &mut Arc<T> = &mut rights[j];
                (compare)(it, pivot) == Ordering::Less
            };
            if less {
                rights.swap(i, j);
                i += 1;
            }
        }
    }

    i += left;
    xs.swap(left, i);
    i
}


fn quicksort<T, F>(xs: &mut [Arc<T>], left: usize, right: usize, compare: &F) where F: Fn(&T, &T) -> Ordering {
  if right - left <= 1 {
    return;
  }

  let pivot = partition(xs, left, right, compare);
  quicksort(xs, left, pivot, compare);
  quicksort(xs, pivot + 1, right, compare);
}
