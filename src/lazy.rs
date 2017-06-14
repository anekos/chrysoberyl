
#[derive(Clone)]
pub struct Lazy<T> {
    item: Option<T>,
}


impl<T> Lazy<T> {
    fn get<F>(&mut self, generate: F) -> &T
    where F: FnOnce() -> T {
        if self.item.is_none() {
            self.item = Some(generate());
        }
        if let Some(ref item) = self.item {
            item
        } else {
            panic!("WTF: item is empty")
        }
    }
}


impl<T> Default for Lazy<T> {
    fn default() -> Self {
        Lazy { item: None }
    }
}
