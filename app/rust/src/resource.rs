use std::ops::Deref;

use crate::runtime::spawn_pinned;
use send_wrapper::SendWrapper;

pub struct Res<T: 'static>(Option<SendWrapper<T>>);

impl<T: 'static> Res<T> {
    pub fn take(mut self) -> T {
        self.0.take().unwrap().take()
    }
}

impl<T: 'static> From<T> for Res<T> {
    fn from(value: T) -> Self {
        Self(Some(SendWrapper::new(value)))
    }
}

impl<T: 'static> Deref for Res<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}

impl<T: 'static> Drop for Res<T> {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            spawn_pinned(async || {
                drop(value);
            });
        }
    }
}
