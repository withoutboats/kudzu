use std::borrow::Borrow;
use std::cmp::Ordering;

use crate::{SkipList, AbstractOrd, QWrapper};

pub struct Map<K, V> {
    inner: SkipList<KeyValue<K, V>>,
}

impl<K: Ord, V> Map<K, V> {
    pub fn new() -> Map<K, V> {
        Map { inner: SkipList::new() }
    }

    pub fn insert(&self, key: K, value: V) -> Option<(K, V)> {
        self.inner.insert(KeyValue(key, value)).map(|KeyValue(k, v)| (k, v))
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
