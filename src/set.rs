use std::borrow::Borrow;
use std::iter::FromIterator;

use crate::{SkipList, QWrapper};
use crate::skiplist::*;

pub struct Set<T> {
    inner: SkipList<T>,
}

impl<T: Ord> Set<T> {
    pub fn new() -> Set<T> {
        Set { inner: SkipList::new() }
    }

    pub fn insert(&self, elem: T) -> Option<(T, &T)> {
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

impl<T: Ord> Extend<T> for Set<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

impl<'a, T: 'a + Ord + Copy> Extend<&'a T> for Set<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.inner.extend(iter);
    }
}

impl<T: Ord> FromIterator<T> for Set<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = Self::new();
        set.extend(iter);
        set
    }
}

#[test]
fn test_collect() {
    let range = 0..100;
    let set: Set<_> = range.clone().collect();
    range.for_each(|i| assert!(set.contains(&i)));
}

#[cfg(feature = "rayon")]
mod parallel {
    use super::Set;
    use rayon::prelude::*;

    impl<T: Ord + Send + Sync> ParallelExtend<T> for Set<T> {
        fn par_extend<I: IntoParallelIterator<Item = T>>(&mut self, iter: I) {
            self.inner.par_extend(iter);
        }
    }

    impl<'a, T: 'a + Ord + Copy + Send + Sync> ParallelExtend<&'a T> for Set<T> {
        fn par_extend<I: IntoParallelIterator<Item = &'a T>>(&mut self, iter: I) {
            self.inner.par_extend(iter);
        }
    }

    impl<T: Ord + Send + Sync> FromParallelIterator<T> for Set<T> {
        fn from_par_iter<I: IntoParallelIterator<Item = T>>(iter: I) -> Self {
            let mut set = Self::new();
            set.par_extend(iter);
            set
        }
    }

    #[test]
    fn test_collect() {
        let range = 0..100;
        let set: Set<_> = range.clone().into_par_iter().collect();
        range
            .into_par_iter()
            .for_each(|i| assert!(set.contains(&i)));
    }
}
