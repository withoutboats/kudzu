mod iter;

use std::alloc;
use std::cmp;
use std::cmp::Ordering::*;
use std::fmt;
use std::mem::{self, ManuallyDrop};
use std::ptr::{self, NonNull};
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::*;

use crate::AbstractOrd;

pub use self::iter::*;

const MAX_HEIGHT: usize = 31;
type Ptr<T>     = Option<NonNull<T>>;
type Lanes<T>   = [AtomicPtr<Node<T>>; 1];// NB: Lanes is actually a variable sized array of lanes,
                                        // containing at least one lane, but possibly as many as
                                        // MAX_HEIGHT.

pub struct SkipList<T> {
    lanes: [AtomicPtr<Node<T>>; MAX_HEIGHT],
}

unsafe impl<T: Send> Send for SkipList<T> { }
unsafe impl<T: Sync> Sync for SkipList<T> { }

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
            lanes: Default::default(),
        }
    }

    pub fn get<'a, U: AbstractOrd<T> + ?Sized>(&'a self, elem: &U) -> Option<&T> {
        let mut lanes: &[AtomicPtr<Node<T>>]    = &self.lanes[..];
        let mut height: usize                   = MAX_HEIGHT;

        'across: while height > 0 {
            'down: for atomic_ptr in lanes {
                let ptr: Ptr<Node<T>> = NonNull::new(atomic_ptr.load(Relaxed));

                match ptr {
                    None        => {
                        height -= 1;
                        continue 'down;
                    }
                    Some(ptr)  => {
                        let node: &'a Node<T> = unsafe { &*ptr.as_ptr() };

                        match elem.cmp(&node.inner.elem) {
                            Equal   => return Some(&node.inner.elem),
                            Less    => {
                                height -= 1;
                                continue 'down;
                            }
                            Greater => {
                                lanes = &node.lanes()[(node.inner.height as usize - height)..];
                                continue 'across;
                            }
                        }
                    }
                }
            }
        }

        return None;
    }

    pub fn insert<'a>(&'a self, elem: T) -> Option<T> {
        let mut elem: ManuallyDrop<T> = ManuallyDrop::new(elem);
        let mut elem_ptr: *const T = &*elem as *const T;
        let mut new_node: Ptr<Node<T>> = None;

        'unlock: loop {
            let mut spots: [(*const AtomicPtr<Node<T>>, *mut Node<T>); MAX_HEIGHT] =
                [(ptr::null(), ptr::null_mut()); MAX_HEIGHT];
            let mut lanes: &'a [AtomicPtr<Node<T>>] = &self.lanes[..];
            let mut height = MAX_HEIGHT;

            'across: while height > 0 {
                'down: for atomic_ptr in lanes {
                    let ptr: Ptr<Node<T>> = NonNull::new(atomic_ptr.load(Relaxed));

                    match ptr {
                        None        => {
                            height -= 1;
                            spots[height] = (atomic_ptr, ptr::null_mut());
                            continue 'down;
                        }
                        Some(ptr)   => unsafe {
                            let node: &'a Node<T> = &*ptr.as_ptr();
                            let elem_ref: &T = &*elem_ptr;

                            match elem_ref.cmp(&node.inner.elem) {
                                Equal   => {
                                    match &mut new_node {
                                        Some(new_node)  => return Some(new_node.as_mut().dealloc()),
                                        None            => return Some(ManuallyDrop::take(&mut elem)),
                                    }
                                }
                                Less    => {
                                    height -= 1;
                                    spots[height] = (atomic_ptr, ptr.as_ptr());
                                    continue 'down;
                                }
                                Greater => {
                                    lanes = &node.lanes()[(node.inner.height as usize - height)..];
                                    continue 'across;
                                }
                            }
                        }
                    }
                }
            }

            let new_node: &Node<T> = unsafe {
                match &new_node {
                    Some(node)  => node.as_ref(),
                    None        => {
                        let node = Node::alloc(ManuallyDrop::take(&mut elem), random_height());
                        elem_ptr = &(*node.as_ptr()).inner.elem;
                        new_node = Some(node);
                        new_node.as_ref().unwrap().as_ref()
                    }
                }
            };

            let new_node_ptr = new_node as *const Node<T> as *mut Node<T>;
            let mut inserted = false;

            'insert: for (&(pred, succ), new) in spots.iter().zip(new_node.lanes().iter().rev()) {
                let pred: &'a AtomicPtr<Node<T>> = unsafe { &*pred };

                new.store(succ, Release);

                if succ == pred.compare_and_swap(succ, new_node_ptr, AcqRel) {
                    inserted = true;
                } else if !inserted {
                    continue 'unlock;
                } else {
                    break 'insert;
                }
            }

            return None;
        }
    }
}

impl<T> SkipList<T> {
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
    fn alloc(elem: T, height: usize) -> NonNull<Node<T>> {
        unsafe {
            let layout = Node::<T>::layout(height);
            let ptr = alloc::alloc_zeroed(layout) as *mut Node<T>;
            (*ptr).inner.height = height as u8;
            ptr::write(&mut (*ptr).inner.elem as *mut T, elem);
            NonNull::new_unchecked(ptr)
        }
    }

    unsafe fn dealloc(&mut self) -> T {
        let layout = Node::<T>::layout(self.inner.height as usize);
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
        let height = self.inner.height as usize;
        unsafe { mem::transmute(LanesPtr { lanes, height }) }
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
            for x in (0..ELEMS).filter(|x| x % THREADS == offset) {
                list.insert(x);
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
