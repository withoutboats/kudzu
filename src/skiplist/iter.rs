use std::marker::PhantomData;
use std::mem;
use std::ptr::{self, NonNull};

use super::{Ptr, Node};

pub(super) struct Nodes<'a, T> {
    ptr: Ptr<Node<T>>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> Nodes<'a, T> {
    pub(super) fn new(ptr: Ptr<Node<T>>) -> Nodes<'a, T> {
        Nodes { ptr, _marker: PhantomData }
    }
}

impl<'a, T> Nodes<'a, T> {
    fn peek(&self) -> Option<&Node<T>> {
        unsafe { mem::transmute(self.ptr) }
    }
}

impl<'a, T> Iterator for Nodes<'a, T> {
    type Item = &'a Node<T>;
    fn next(&mut self) -> Option<&'a Node<T>> {
        unsafe {
            let ptr: NonNull<Node<T>> = self.ptr.take()?;
            {
                let node: &Node<T> = ptr.as_ref();
                self.ptr = node.next();
            }
            mem::transmute(ptr)
        }
    }
}

pub(super) struct NodesMut<'a, T> {
    ptr: Ptr<Node<T>>,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> NodesMut<'a, T> {
    pub(super) fn new(ptr: Ptr<Node<T>>) -> NodesMut<'a, T> {
        NodesMut { ptr, _marker: PhantomData }
    }
}

impl<'a, T> Iterator for NodesMut<'a, T> {
    type Item = &'a mut Node<T>;
    fn next(&mut self) -> Option<&'a mut Node<T>> {
        unsafe {
            let ptr: NonNull<Node<T>> = self.ptr.take()?;
            {
                let node: &Node<T> = ptr.as_ref();
                self.ptr = node.next();
            }
            mem::transmute(ptr)
        }
    }
}

pub struct Elems<'a, T> {
    pub(super) nodes: Nodes<'a, T>
}

impl<'a, T> Elems<'a, T> {
    pub(crate) fn peek(&self) -> Option<&T> {
        self.nodes.peek().map(|node| &node.inner.elem)
    }
}

impl<'a, T> Iterator for Elems<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.next().map(|node| &node.inner.elem)
    }
}

pub struct ElemsMut<'a, T> {
    pub(super) nodes: NodesMut<'a, T>
}

impl<'a, T> Iterator for ElemsMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.next().map(|node| &mut node.inner.elem)
    }
}

pub struct IntoElems<T> {
    pub(super) ptr: Ptr<Node<T>>,
}

impl<T> Iterator for IntoElems<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut ptr = self.ptr.take()?;
            let node: &mut Node<T> = ptr.as_mut();
            self.ptr = node.next();
            let elem = ptr::read(&mut node.inner.elem as *mut T);
            node.dealloc();
            Some(elem)
        }
    }
}
