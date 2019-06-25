use std::borrow::Borrow;
use std::cmp::Ordering;
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

    pub fn difference<'a>(&'a self, other: &'a Self) -> Difference<'a, T> {
        Difference {
            left: self.inner.elems(),
            right: other.inner.elems(),
        }
    }

    pub fn symmetric_difference<'a>(&'a self, other: &'a Self) -> SymmetricDifference<'a, T> {
        SymmetricDifference {
            left: self.inner.elems(),
            right: other.inner.elems(),
        }
    }

    pub fn intersection<'a>(&'a self, other: &'a Self) -> Intersection<'a, T> {
        Intersection {
            left: self.inner.elems(),
            right: other.inner.elems(),
        }
    }

    pub fn union<'a>(&'a self, other: &'a Self) -> Union<'a, T> {
        Union {
            left: self.inner.elems(),
            right: other.inner.elems(),
        }
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

pub struct Difference<'a, T> {
    left: Elems<'a, T>,
    right: Elems<'a, T>,
}

impl<'a, T: Ord> Iterator for Difference<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        // Find the next left that's less than the next right
        'left: loop {
            // When the left is empty, we're done
            let left = self.left.next()?;
            // Peek the next right and compare
            while let Some(right) = self.right.peek() {
                match left.cmp(right) {
                    Ordering::Less => break,
                    Ordering::Equal => {
                        // Consume right and get a new left
                        self.right.next();
                        continue 'left;
                    }
                    Ordering::Greater => {
                        // Consume right, so we'll peek a new one
                        self.right.next();
                    }
                }
            }
            // Found Less, or right was None
            return Some(left);
        }
    }
}

pub struct SymmetricDifference<'a, T> {
    left: Elems<'a, T>,
    right: Elems<'a, T>,
}

impl<'a, T: Ord> Iterator for SymmetricDifference<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        // Find the next exclusive value from the left or right
        loop {
            return match (self.left.peek(), self.right.peek()) {
                // Both are empty
                (None, None) => None,
                // The right is empty -- consume the left
                (Some(_), None) => self.left.next(),
                // The left is empty -- consume the right
                (None, Some(_)) => self.right.next(),
                // Both have values -- return the lesser
                (Some(left), Some(right)) => match left.cmp(right) {
                    Ordering::Less => self.left.next(),
                    Ordering::Greater => self.right.next(),
                    Ordering::Equal => {
                        // Discard equal values and try again
                        self.left.next();
                        self.right.next();
                        continue;
                    }
                },
            };
        }
    }
}

pub struct Intersection<'a, T> {
    left: Elems<'a, T>,
    right: Elems<'a, T>,
}

impl<'a, T: Ord> Iterator for Intersection<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        // Find the next value that exists in both
        loop {
            // When either is empty, we're done
            let left = self.left.peek()?;
            let right = self.right.peek()?;
            match left.cmp(right) {
                // Discard the lesser inequal value
                Ordering::Less => self.left.next(),
                Ordering::Greater => self.right.next(),
                Ordering::Equal => {
                    // Consume both, returning the left
                    self.right.next();
                    return self.left.next();
                }
            };
        }
    }
}

pub struct Union<'a, T> {
    left: Elems<'a, T>,
    right: Elems<'a, T>,
}

impl<'a, T: Ord> Iterator for Union<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        // Find the next value from either
        match (self.left.peek(), self.right.peek()) {
            // Both are empty
            (None, None) => None,
            // The right is empty -- consume the left
            (Some(_), None) => self.left.next(),
            // The left is empty -- consume the right
            (None, Some(_)) => self.right.next(),
            (Some(left), Some(right)) => match left.cmp(right) {
                // Return the lesser inequal value
                Ordering::Less => self.left.next(),
                Ordering::Greater => self.right.next(),
                Ordering::Equal => {
                    // Consume both, returning the left
                    self.right.next();
                    self.left.next()
                }
            },
        }
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

#[cfg(test)]
mod test_iterators {
    use super::Set;
    use std::collections::BTreeSet;

    static SINGLES: &[u32] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    static EVENS: &[u32] = &[0, 2, 4, 6, 8, 10, 12, 14, 16, 18];
    static ODDS: &[u32] = &[1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
    static PRIMES: &[u32] = &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29];
    static SETS: &[&[u32]] = &[SINGLES, EVENS, ODDS, PRIMES];

    fn collect<'a, I, C>(iter: I) -> C
    where
        I: IntoIterator<Item = &'a u32>,
        C: std::iter::FromIterator<u32>,
    {
        iter.into_iter().copied().collect()
    }

    fn compare<F, G>(f: F, g: G)
    where
        F: Fn(&Set<u32>, &Set<u32>) -> Vec<u32>,
        G: Fn(&BTreeSet<u32>, &BTreeSet<u32>) -> Vec<u32>,
    {
        for &left in SETS {
            let a1: Set<_> = collect(left);
            let b1: BTreeSet<_> = collect(left);

            for &right in SETS {
                let a2: Set<_> = collect(right);
                let b2: BTreeSet<_> = collect(right);

                assert_eq!(f(&a1, &a2), g(&b1, &b2));
            }
        }
    }

    #[test]
    fn difference() {
        compare(
            |a, b| collect(a.difference(b)),
            |a, b| collect(a.difference(b)),
        );
    }

    #[test]
    fn symmetric_difference() {
        compare(
            |a, b| collect(a.symmetric_difference(b)),
            |a, b| collect(a.symmetric_difference(b)),
        );
    }

    #[test]
    fn intersection() {
        compare(
            |a, b| collect(a.intersection(b)),
            |a, b| collect(a.intersection(b)),
        );
    }

    #[test]
    fn union() {
        compare(
            |a, b| collect(a.union(b)),
            |a, b| collect(a.union(b))
        );
    }
}
