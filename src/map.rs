use std::borrow::Borrow;
use std::cmp::Ordering;
use std::iter::FromIterator;

use crate::{SkipList, AbstractOrd, QWrapper};

pub struct Map<K, V> {
    inner: SkipList<KeyValue<K, V>>,
}

impl<K: Ord, V> Map<K, V> {
    pub fn new() -> Map<K, V> {
        Map { inner: SkipList::new() }
    }

    pub fn insert(&self, key: K, value: V) -> Option<(K, V, &K, &V)> {
        self.inner.insert(KeyValue(key, value)).map(|(KeyValue(k, v), kv)| (k, v, &kv.0, &kv.1))
    }

    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        Q: Ord + ?Sized,
        K: Borrow<Q>,
    {
        self.get(key).is_some()
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: Ord + ?Sized,
        K: Borrow<Q>,
    {
        self.inner.get(QWrapper::new(key)).map(|KeyValue(_, v)| v)
    }

    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        Q: Ord + ?Sized,
        K: Borrow<Q>,
    {
        self.inner.get(QWrapper::new(key)).map(|KeyValue(k, v)| (k, v))
    }
}

struct KeyValue<K, V>(K, V);

impl<K: Ord, V> AbstractOrd<KeyValue<K, V>> for KeyValue<K, V> {
    fn cmp(&self, rhs: &KeyValue<K, V>) -> Ordering {
        Ord::cmp(&self.0, &rhs.0)
    }
}

impl<K, V, Q> AbstractOrd<KeyValue<K, V>> for QWrapper<Q>
where
    K: Ord + Borrow<Q>,
    Q: Ord + ?Sized,
{
    fn cmp(&self, rhs: &KeyValue<K, V>) -> Ordering {
        Ord::cmp(&self.0, rhs.0.borrow())
    }
}

impl<K: Ord, V> Extend<(K, V)> for Map<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let iter = iter.into_iter().map(|(key, value)| KeyValue(key, value));
        self.inner.extend(iter);
    }
}

impl<'a, K: Ord + Copy, V: Copy> Extend<(&'a K, &'a V)> for Map<K, V> {
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        let iter = iter.into_iter().map(|(&key, &value)| KeyValue(key, value));
        self.inner.extend(iter);
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for Map<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = Self::new();
        map.extend(iter);
        map
    }
}

#[cfg(feature = "rayon")]
mod parallel {
    use super::{KeyValue, Map};
    use rayon::prelude::*;

    impl<K, V> ParallelExtend<(K, V)> for Map<K, V>
    where
        K: Ord + Send + Sync,
        V: Send + Sync,
    {
        fn par_extend<I: IntoParallelIterator<Item = (K, V)>>(&mut self, iter: I) {
            let iter = iter
                .into_par_iter()
                .map(|(key, value)| KeyValue(key, value));
            self.inner.par_extend(iter);
        }
    }

    impl<'a, K, V> ParallelExtend<(&'a K, &'a V)> for Map<K, V>
    where
        K: Ord + Copy + Send + Sync,
        V: Copy + Send + Sync,
    {
        fn par_extend<I: IntoParallelIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
            let iter = iter
                .into_par_iter()
                .map(|(&key, &value)| KeyValue(key, value));
            self.inner.par_extend(iter);
        }
    }

    impl<K, V> FromParallelIterator<(K, V)> for Map<K, V>
    where
        K: Ord + Send + Sync,
        V: Send + Sync,
    {
        fn from_par_iter<I: IntoParallelIterator<Item = (K, V)>>(iter: I) -> Self {
            let mut map = Self::new();
            map.par_extend(iter);
            map
        }
    }
}
