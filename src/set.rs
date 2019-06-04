use std::borrow::Borrow;

use crate::{SkipList, QWrapper};
use crate::skiplist::*;

pub struct Set<T> {
    inner: SkipList<T>,
}

impl<T: Ord> Set<T> {
    pub fn new() -> Set<T> {
        Set { inner: SkipList::new() }
    }

    pub fn insert(&self, elem: T) -> Option<T> {
        self.inner.insert(elem)
    }

    pub fn contains<Q>(&self, value: &Q) -> bool
    where
        Q: Ord + ?Sized,
        T: Borrow<Q>,
    {
        self.get(value).is_some()
    }

    pub fn get<Q>(&self, value: &Q) -> Option<&T>
    where
        Q: Ord + ?Sized,
        T: Borrow<Q>,
    {
        self.inner.get(QWrapper::new(value))
    }

    pub fn iter(&self) -> Iter<'_, T> {
        IntoIterator::into_iter(self)
    }
}

impl<T> IntoIterator for Set<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { inner: self.inner.into_elems() }
    }
}

impl<'a, T> IntoIterator for &'a Set<T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;
    fn into_iter(self) -> Iter<'a, T> {
        Iter { inner: self.inner.elems() }
    }
}

pub struct IntoIter<T> {
    inner: IntoElems<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct Iter<'a, T> {
    inner: Elems<'a, T>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}
