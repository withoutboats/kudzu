mod get;
mod insert;
mod iter;

use std::alloc;
use std::cmp;
use std::fmt;
use std::iter::FromIterator;
use std::mem;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, AtomicU8};
use std::sync::atomic::Ordering::Relaxed;

use crate::AbstractOrd;

pub use self::iter::*;

const MAX_HEIGHT: usize = 31;
type Ptr<T>     = Option<NonNull<T>>;
type Lanes<T>   = [AtomicPtr<Node<T>>; 1];// NB: Lanes is actually a variable sized array of lanes,
                                        // containing at least one lane, but possibly as many as
                                        // MAX_HEIGHT.

pub struct SkipList<T> {
    current_height: AtomicU8,
    lanes: [AtomicPtr<Node<T>>; MAX_HEIGHT],
}

unsafe impl<T: Send + Sync> Send for SkipList<T> { }
unsafe impl<T: Send + Sync> Sync for SkipList<T> { }

#[repr(C)] // NB: repr(C) necessary to avoid reordering lanes field, which must be the tail
struct Node<T> {
    inner: InnerNode<T>,
    lanes: Lanes<T>,
}

// NB: To allow optimizing repr of these fields
struct InnerNode<T> {
    elem: T,
    height: u8,
}

impl<T: AbstractOrd<T>> SkipList<T> {
    pub fn new() -> SkipList<T> {
        SkipList {
            current_height: AtomicU8::new(8),
            lanes: Default::default(),
        }
    }

    pub fn insert<'a>(&'a self, elem: T) -> Option<(T, &'a T)> {
        insert::insert(&self.lanes[..], elem, &self.current_height)
    }
}

impl<T> SkipList<T> {
    fn lanes(&self) -> &[AtomicPtr<Node<T>>] {
        let init = MAX_HEIGHT - self.current_height.load(Relaxed) as usize;
        &self.lanes[init..]
    }

    pub fn get<'a, U: AbstractOrd<T> + ?Sized>(&'a self, elem: &U) -> Option<&T> {
        get::get(self.lanes(), elem)
    }

    pub fn elems(&self) -> Elems<'_, T> {
        Elems { nodes: self.nodes() }
    }

    pub fn elems_mut(&mut self) -> ElemsMut<'_, T> {
        ElemsMut { nodes: self.nodes_mut() }
    }

    pub fn into_elems(self) -> IntoElems<T> {
        let ptr = self.first();
        mem::forget(self);
        IntoElems { ptr }
    }

    fn nodes(&self) -> Nodes<'_, T> {
        Nodes::new(self.first())
    }

    fn nodes_mut(&mut self) -> NodesMut<'_, T> {
        NodesMut::new(self.first())
    }

    fn first(&self) -> Ptr<Node<T>> {
        NonNull::new(self.lanes[MAX_HEIGHT - 1].load(Relaxed))
    }
}

impl<T> Node<T> {
    fn alloc(elem: T, max_height: &AtomicU8) -> NonNull<Node<T>> {
        let height = random_height();
        max_height.fetch_max(height as u8, Relaxed);
        unsafe {
            let layout = Node::<T>::layout(height);
            let ptr = alloc::alloc_zeroed(layout) as *mut Node<T>;
            (*ptr).inner.height = height as u8;
            ptr::write(&mut (*ptr).inner.elem as *mut T, elem);
            NonNull::new_unchecked(ptr)
        }
    }

    unsafe fn dealloc(&mut self) -> T {
        let layout = Node::<T>::layout(self.height());
        let elem = ptr::read(&mut self.inner.elem);
        alloc::dealloc(self as *mut Node<T> as *mut u8, layout);
        elem
    }

    fn next(&self) -> Ptr<Node<T>> {
        NonNull::new(self.lanes().last().unwrap().load(Relaxed))
    }

    fn lanes(&self) -> &[AtomicPtr<Node<T>>] {
        #[repr(C)]
        struct LanesPtr<T> {
            lanes: *const Lanes<T>,
            height: usize,
        }

        let lanes = &self.lanes as *const Lanes<T>;
        let height = self.height();
        unsafe { mem::transmute(LanesPtr { lanes, height }) }
    }

    fn height(&self) -> usize {
        self.inner.height as usize
    }

    fn layout(height: usize) -> alloc::Layout {
        let size = ((height + 1) * mem::size_of::<usize>()) + mem::size_of::<T>();
        let align = cmp::max(mem::align_of::<T>(), mem::align_of::<usize>());
        unsafe {
            alloc::Layout::from_size_align_unchecked(size, align)
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for SkipList<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.nodes()).finish()
    }
}

impl<T: fmt::Debug> fmt::Debug for Node<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Node")
            .field("elem", &self.inner.elem)
            .field("height", &self.inner.height)
            .field("lanes", &self.lanes())
            .finish()
    }
}

impl<T> Drop for SkipList<T> {
    fn drop(&mut self) {
        // TODO call destructors
        for node in self.nodes_mut() {
            unsafe { drop(node.dealloc()) }
        }
    }
}

impl<T: AbstractOrd<T>> Extend<T> for SkipList<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|elem| {
            self.insert(elem);
        });
    }
}

impl<'a, T: AbstractOrd<T> + Copy> Extend<&'a T> for SkipList<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        iter.into_iter().for_each(|&elem| {
            self.insert(elem);
        });
    }
}

impl<T: AbstractOrd<T>> FromIterator<T> for SkipList<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut list = Self::new();
        list.extend(iter);
        list
    }
}

fn random_height() -> usize {
    const MASK: u32 = 1 << (MAX_HEIGHT - 1);
    1 + (rand::random::<u32>() | MASK).trailing_zeros() as usize
}

#[test]
fn test() {
    #[derive(Debug, Ord, PartialOrd, Eq, PartialEq)] struct DropInt(i32);
    impl Drop for DropInt { fn drop(&mut self) {
        println!("Dropping {}", self.0);
    } }
    let list = SkipList::new();
    list.insert(DropInt(1));
    list.insert(DropInt(3));
    list.insert(DropInt(0));
    list.insert(DropInt(5));
    list.insert(DropInt(2));
    assert!(list.insert(DropInt(3)).is_some());
    list.insert(DropInt(4));
}

#[test]
fn test_concurrent() {
    const THREADS: i32 = 16;
    const ELEMS: i32 = 100_000;
    let list = std::sync::Arc::new(SkipList::new());
    let mut handles = vec![];
    for offset in 0..THREADS {
        let list = list.clone();
        handles.push(std::thread::spawn(move || {
            if offset % 2 == 0 {
                for x in (0..ELEMS).filter(|x| x % THREADS == offset) {
                    list.insert(x);
                }
            } else {
                for x in (0..ELEMS).filter(|x| x % THREADS == offset).rev() {
                    list.insert(x);
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    for (&elem, expected) in list.elems().zip(0..ELEMS) {
        assert_eq!(elem, expected);
    }
}
