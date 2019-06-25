use std::borrow::Borrow;
use std::cmp::Ordering;
use std::iter::FromIterator;

use crate::{SkipList, AbstractOrd, QWrapper};
use crate::skiplist::*;

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

    pub fn iter(&self) -> Iter<'_, K, V> {
        IntoIterator::into_iter(self)
    }

    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IntoIterator::into_iter(self)
    }

    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut { inner: self.iter_mut() }
    }
}

impl<K, V> IntoIterator for Map<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = (K, V);
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { inner: self.inner.into_elems() }
    }
}

impl<'a, K, V> IntoIterator for &'a Map<K, V> {
    type IntoIter = Iter<'a, K, V>;
    type Item = (&'a K, &'a V);
    fn into_iter(self) -> Self::IntoIter {
        Iter { inner: self.inner.elems() }
    }
}

impl<'a, K, V> IntoIterator for &'a mut Map<K, V> {
    type IntoIter = IterMut<'a, K, V>;
    type Item = (&'a K, &'a mut V);
    fn into_iter(self) -> Self::IntoIter {
        IterMut { inner: self.inner.elems_mut() }
    }
}

pub struct IntoIter<K, V> {
    inner: IntoElems<KeyValue<K, V>>,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|KeyValue(k, v)| (k, v))
    }
}

pub struct Iter<'a, K: 'a, V: 'a> {
    inner: Elems<'a, KeyValue<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|KeyValue(k, v)| (k, v))
    }
}

pub struct Keys<'a, K: 'a, V: 'a> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }
}

pub struct Values<'a, K: 'a, V: 'a> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }
}

pub struct IterMut<'a, K: 'a, V: 'a> {
    inner: ElemsMut<'a, KeyValue<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|KeyValue(k, v)| (&*k, v))
    }
}

pub struct ValuesMut<'a, K: 'a, V: 'a> {
    inner: IterMut<'a, K, V>,
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
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
