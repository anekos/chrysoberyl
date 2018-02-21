
use std::sync::Mutex;
use std::cell::RefCell;



pub struct Lazy<T> {
    inner: Mutex<RefCell<Inner<T>>>,
}

enum Inner<T> {
    Initial,
    Evaludated(T),
}


impl<T> Lazy<T> {
    pub fn new() -> Self {
        Lazy { inner: Mutex::new(RefCell::new(Inner::Initial)) }
    }

    pub fn get<F, G, U>(&self, ctor: F, fetch: G) -> U where F: FnOnce() -> T, G: FnOnce(&T) -> U {
        self.evaluate(ctor);

        let inner = self.inner.lock().unwrap();
        let inner = inner.borrow();

        match *inner {
            Inner::Evaludated(ref v) => {
                fetch(v)
            },
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
