
use entry::Key;



#[derive(Default)]
pub struct History {
    backwards: Vec<Key>,
    forwards: Vec<Key>,
}


impl History {
    pub fn record(&mut self, key: Key) {
        self.backwards.push(key);
        self.forwards.clear();
    }

    pub fn forward(&mut self) -> Option<&Key> {
        move_entry(&mut self.forwards, &mut self.backwards)
    }

    pub fn backward(&mut self) -> Option<&Key> {
        move_entry(&mut self.backwards, &mut self.forwards)
    }

    pub fn go(&mut self, forward: bool) -> Option<&Key> {
        if forward {
            self.forward()
        } else {
            self.backward()
        }
    }
}


fn move_entry<'a>(from: &'a mut Vec<Key>, to: &'a mut Vec<Key>) -> Option<&'a Key> {
    from.pop().map(move |it| {
        to.push(it);
        to.last().unwrap()
    })
}
