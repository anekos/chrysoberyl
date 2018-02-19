
use std::sync::{Arc, Mutex};
use std::cell::RefCell;



#[derive(Clone)]
pub struct Lazy<T> {
    inner: Arc<Mutex<RefCell<Inner<T>>>>,
}

#[derive(Clone)]
enum Inner<T> {
    Initial,
    Evaludated(T),
}


impl<T> Lazy<T> {
    pub fn new() -> Self {
        Lazy { inner: Arc::new(Mutex::new(RefCell::new(Inner::Initial))) }
    }

    pub fn get<F>(&self, ctor: F) -> &T where F: FnOnce() -> T {
        self.evaluate(ctor);

        let inner = self.inner.lock().unwrap();

        let inner = unsafe { inner.as_ptr().as_ref().unwrap() };
        match *inner {
            Inner::Evaludated(ref v) => v,
            _ => panic!("WTF"),
        }
    }

    pub fn evaluate<F>(&self, ctor: F) where F: FnOnce() -> T {
        let inner = self.inner.lock().unwrap();

        if let Inner::Evaludated(_) = *inner.borrow() {
            return;
        }

        let mut inner = inner.borrow_mut();
        *inner = Inner::Evaludated(ctor());
    }
}
