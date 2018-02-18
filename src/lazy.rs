
use std::cell::RefCell;



#[derive(Clone)]
pub struct Lazy<T> {
    inner: RefCell<Inner<T>>,
}

#[derive(Clone)]
enum Inner<T> {
    Initial,
    Evaludated(T),
}


impl<T> Lazy<T> {
    pub fn new() -> Self {
        Lazy { inner: RefCell::new(Inner::Initial) }
    }

    pub fn get<F>(&self, ctor: F) -> &T where F: FnOnce() -> T {
        self.evaluate(ctor);
        let inner = unsafe { self.inner.as_ptr().as_ref().unwrap() };
        match *inner {
            Inner::Evaludated(ref v) => v,
            _ => panic!("WTF"),
        }
    }

    pub fn evaluate<F>(&self, ctor: F) where F: FnOnce() -> T {
        if let Inner::Evaludated(_) = *self.inner.borrow() {
            return;
        }

        let mut inner = self.inner.borrow_mut();
        *inner = Inner::Evaludated((ctor)());
    }
}
