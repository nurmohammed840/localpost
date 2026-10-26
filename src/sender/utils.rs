#![cfg(test)]

use std::collections::VecDeque;

pub trait VecExt<T> {
    fn find_remove(&mut self, f: impl FnMut(&T) -> bool) -> Option<T>;
}

impl<T> VecExt<T> for VecDeque<T> {
    fn find_remove(&mut self, f: impl FnMut(&T) -> bool) -> Option<T> {
        let index = self.iter().position(f)?;
        self.remove(index)
    }
}

impl<T> VecExt<T> for Vec<T> {
    fn find_remove(&mut self, f: impl FnMut(&T) -> bool) -> Option<T> {
        self.iter().position(f).map(|index| self.remove(index))
    }
}

pub trait IteratorExt: Iterator + Sized {
    fn extend_into<E: Extend<Self::Item>>(self, collection: &mut E) -> &mut E
    where
        Self: Sized,
    {
        collection.extend(self);
        collection
    }
}

impl<I: Iterator> IteratorExt for I {}
