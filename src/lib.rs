pub use typed_builder_proc_macros::*;

pub trait BuilderOptional<T> {
    fn into_value<F: FnOnce() -> Option<T>>(self, default: F) -> T;
}

impl<T> BuilderOptional<T> for () {
    fn into_value<F: FnOnce() -> Option<T>>(self, default: F) -> T {
        default().unwrap()
    }
}

impl<T> BuilderOptional<T> for (T,) {
    fn into_value<F: FnOnce() -> Option<T>>(self, _: F) -> T {
        self.0
    }
}
