use std::cmp::Ordering::*;
use std::ptr::NonNull;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering::Acquire;

use crate::AbstractOrd;
use super::{Node, Ptr};

pub(super) fn get<'a, T, U>(mut lanes: &'a [AtomicPtr<Node<T>>], elem: &U) -> Option<&'a T>
    where U: AbstractOrd<T> + ?Sized
{
    let mut height = lanes.len();

    'across: while height > 0 {
        'down: for atomic_ptr in lanes {
            let ptr: Ptr<Node<T>> = NonNull::new(atomic_ptr.load(Acquire));

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
                            lanes = &node.lanes()[(node.height() - height)..];
                            continue 'across;
                        }
                    }
                }
            }
        }
    }

    return None;
}
