
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::slice;

use rand::{thread_rng, Rng, ThreadRng};



pub struct FilterableVec<T: Clone + Hash + Eq + Sized> {
    original: Vec<Rc<T>>,
    filtered: Vec<Rc<T>>,
    indices: HashMap<Rc<T>, usize>,
    rng: ThreadRng,
    pred: Option<Box<FnMut(&mut T) -> bool>>,
}


impl<T: Clone + Hash + Eq + Sized + Ord> FilterableVec<T> {
    pub fn new() -> Self {
        FilterableVec {
            original: vec![],
            filtered: vec![],
            indices: HashMap::new(),
            rng: thread_rng(),
            pred: None,
        }
    }

    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    pub fn contains(&self, entry: &T) -> bool {
        self.get_index(entry).is_some()
    }

    pub fn get_index(&self, entry: &T) -> Option<usize> {
        self.indices.get(entry).map(|it| *it)
    }

    pub fn iter(&self) -> slice::Iter<Rc<T>> {
        self.filtered.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Rc<T>> {
        self.filtered.get(index)
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
        self.indices.clear();
    }

    pub fn sort(&mut self) {
        self.original.sort();
        self.filter();
        self.reset_indices();
    }

    // Shuffle **original** entries
    pub fn shuffle(&mut self) {
        let mut source = self.original.clone();
        let mut buffer = source.as_mut_slice();
        self.rng.shuffle(&mut buffer);
        self.original = buffer.to_vec();

        self.filter();
        self.reset_indices();
    }

    // FIXME
    pub fn insert(&mut self, _: usize, _: Rc<T>) {
        not_implemented!();
        // self.original.insert(index, entry.clone());
        // if (self.pred)(Rc::make_mut(&mut entry)) {
        //     self.filtered.insert(index, entry);
        // }
    }

    pub fn extend_from_slice(&mut self, entries: &[Rc<T>]) {
        self.original.extend_from_slice(entries);
        let targets = if let Some(ref mut pred) = self.pred {
            let mut entries = entries.to_vec();
            let mut targets = vec![];
            for mut entry in entries.iter_mut() {
                if (pred)(Rc::make_mut(&mut entry)) {
                    targets.push(entry.clone());
                }
            }
            targets
        } else {
            self.filtered.extend_from_slice(entries);
            self.reset_indices(); // FXIME Optimize
            return;
        };

        for target in targets {
            self.push_filtered(target);
        }
    }

    pub fn remove(&mut self, index: usize) -> Rc<T> {
        self.original.remove(index);
        let result = self.filtered.remove(index);
        self.reset_indices();
        result
    }

    pub fn push(&mut self, mut entry: Rc<T>) {
        self.original.push(entry.clone());
        let ok = if let Some(ref mut pred) = self.pred {
            (pred)(Rc::make_mut(&mut entry))
        } else {
            true
        };
        if ok {
            self.push_filtered(entry.clone());
        }
    }

    pub fn update_filter(&mut self, pred: Option<Box<FnMut(&mut T) -> bool>>) {
        self.pred = pred;
        self.filter();
    }

    pub fn filter(&mut self) {
        if let Some(ref mut pred) = self.pred {
            self.filtered = vec![];
            for mut entry in self.original.iter_mut() {
                if (pred)(Rc::make_mut(&mut entry)) {
                     self.filtered.push(entry.clone());
                }
            }
        } else {
            self.filtered = self.original.clone();
        }
        self.reset_indices();
    }

    fn push_filtered(&mut self, entry: Rc<T>) {
        self.indices.insert(entry.clone(), self.filtered.len());
        self.filtered.push(entry);
    }

    fn reset_indices(&mut self) {
        for (index, entry) in self.filtered.iter().enumerate() {
            self.indices.insert(entry.clone(), index);
        }
    }
}
