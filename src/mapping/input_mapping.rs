
use std::collections::{VecDeque, HashMap};

use crate::key::{Key, key_sequence_to_string};
use crate::size::{CoordPx, Region};



type OperationCode = Vec<String>;

pub struct InputMapping {
    pub depth: usize,
    pub table: HashMap<Key, Node>
}

pub enum Node {
    Leaf(LeafNode),
    Sub(Box<InputMapping>),
}

pub struct LeafNode {
    pub entries: Vec<WithRegion>
}

pub struct WithRegion {
    pub operation: OperationCode,
    pub region: Option<Region>,
}

pub struct InputHistory {
    entries: VecDeque<Key>,
}


impl InputMapping {
    pub fn new() -> Self {
        InputMapping { depth: 1, table: HashMap::new() }
    }

    pub fn register(&mut self, keys: &[Key], region: Option<Region>, operation: OperationCode) {
        self._register(keys, region, operation);
        self.update_depth();
    }

    fn _register(&mut self, keys: &[Key], region: Option<Region>, operation: OperationCode) {
        use self::Node::*;

        if let Some((head, tail)) = keys.split_first() {
            if let Some(ref mut entry) = self.table.get_mut(head) {
                match **entry {
                    Sub(_) if tail.is_empty() =>
                        **entry = new_mapping_entry(tail, region, operation),
                    Sub(ref mut sub) =>
                        sub.register(tail, region, operation),
                    Leaf(ref mut leaf_node) if tail.is_empty() =>
                        leaf_node.register(region, operation),
                    Leaf(_) =>
                        **entry = new_mapping_entry(tail, region, operation),
                }
                return
            }
            self.table.insert(head.clone(), new_mapping_entry(tail, region, operation));
        } else {
            panic!("Empty key sequence");
        }
    }

    pub fn unregister(&mut self, keys: &[Key], region: &Option<Region>) {
        use self::Node::*;

        if_let_some!((head, tail) = keys.split_first());
        let tail = tail.to_vec();

        let do_remove = {
            if_let_some!(entry = self.table.get_mut(head));
            match *entry {
                Sub(ref mut sub) if !tail.is_empty() =>
                    return sub.unregister(&tail, region),
                Sub(_) =>
                    return,
                Leaf(ref mut leaf_node) =>
                    leaf_node.unregister(region),
            }
        };

        if do_remove {
            self.table.remove(head);
        }
    }

    pub fn matched(&self, history: &InputHistory, coord: CoordPx, width: i32, height: i32) -> Option<(String, Vec<String>)> {
        let entries = &history.entries;
        let len = entries.len();
        for i in 0..len {
            let mut mapping = self;
            let mut inputs = vec![];
            for (j, input) in entries.iter().enumerate().take(len).skip(i) {
                let end = j == len - 1;
                if let Some(entry) = mapping.table.get(input) {
                    match *entry {
                        Node::Sub(ref sub) => {
                            mapping = sub;
                            inputs.push(input.clone());
                        }
                        Node::Leaf(ref leaf_node) if end => {
                            if let Some(matched) = leaf_node.matched(coord, width, height) {
                                inputs.push(input.clone());
                                let inputs = key_sequence_to_string(&inputs);
                                return Some((inputs, matched));
                            }
                        }
                        _ =>
                            return None,
                    }
                }
            }
        }
        None
    }

    pub fn update_depth(&mut self) {
        let sub_max = self.table.iter().map(|(_, entry)| {
            match *entry {
                Node::Sub(ref sub) => sub.depth,
                _ => 0
            }
        }).max();

        self.depth = sub_max.unwrap_or(0) + 1;
    }
}


impl InputHistory {
    pub fn new() -> InputHistory {
        InputHistory { entries: VecDeque::new() }
    }

    pub fn push(&mut self, key: Key, depth: usize) {
        self.entries.push_back(key);
        while depth < self.entries.len() {
            self.entries.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}


fn new_mapping_entry(keys: &[Key], region: Option<Region>, operation: OperationCode) -> Node {
    if !keys.is_empty() {
        let mut result = InputMapping { depth: 1, table: HashMap::new() };
        result.register(keys, region, operation);
        Node::Sub(Box::new(result))
    } else {
        Node::Leaf(LeafNode::new(operation.to_vec(), region))
    }
}


impl LeafNode {
    fn new(operation: OperationCode, region: Option<Region>) -> Self {
        let entry = WithRegion { operation, region };
        LeafNode { entries: vec![entry] }
    }

    pub fn register(&mut self, region: Option<Region>, operation: OperationCode) {
        let entry = WithRegion { operation, region };
        self.entries.retain(|it| it.region != region);
        self.entries.push(entry);
    }

    /**
     * If `entries` comes empty, return `true`
     */
    pub fn unregister(&mut self, region: &Option<Region>) -> bool {
        self.entries.retain(|it| it.region != *region);
        self.entries.is_empty()
    }

    pub fn matched(&self, coord: CoordPx, width: i32, height: i32) -> Option<OperationCode> {
        let mut found = None;

        for entry in &self.entries {
            if let Some(area) = entry.region {
                if area.contains(coord.x, coord.y, width, height) {
                    found = Some(entry.operation.clone());
                    break;
                }
            } else if found.is_none() {
                found = Some(entry.operation.clone());
            }
        }

        found
    }
}
