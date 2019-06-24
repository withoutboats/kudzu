use std::cmp::Ordering::*;
use std::mem::ManuallyDrop;
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, AtomicU8};
use std::sync::atomic::Ordering::{Relaxed, AcqRel, Release};

use crate::AbstractOrd;
use super::{Ptr, Node, MAX_HEIGHT};

pub(super) fn insert<'a, T>(mut init_lanes: &'a [AtomicPtr<Node<T>>], elem: T, max_height: &AtomicU8)
    -> Option<T>
where T: AbstractOrd<T>
{
    // This wonky memory set up is necessary to handle retry iteration: we do
    // not know we need to retry the insertion until after we have already
    // allocated a node for this element. We are faced with a dilemma because
    // of this retry issue:
    //
    //  - The first time searching, we do not know if we need to insert. To
    //    avoid unnecessary allocations, we do not allocate a node for this
    //    element until the first search has turned up empty.
    //  - When we discover we need to retry, we have already allocated the
    //    node for the element, moving the element into the heap. We do not
    //    want to deallocate it for the search, because we would likely need
    //    to allocate it again once we find that the element is still not
    //    present.
    //
    // For this reason, we access the element during search through the
    // "elem_ptr" variable, which could point either to the element in the
    // stack or the element in the heap. We manage dropping/forgetting the elem
    // correctly using a ManuallyDrop wrapper. The `new_node` pointer is used
    // to track if the node has been allocated (it has if it is non-null).
    let mut elem: ManuallyDrop<T> = ManuallyDrop::new(elem);
    let mut elem_ptr: NonNull<T> = NonNull::from(&*elem);
    let mut new_node: Ptr<Node<T>> = None;

    // This tracks how many lanes we have inserted the node into, during
    // retry attempts.
    let mut inserted = 0;

    let mut highest_insert = init_lanes.len();

    'retry: loop {
        let mut lanes = init_lanes;
        let mut height = lanes.len();

        // The immediate predecessor and successor of this element in each
        // lane of the skiplist. The predecessor pointer is a pointer to the
        // actual AtomicPtr in that lane of that node, which will be set to
        // point to this element. The successor pointer is just the address
        // of the successor node, which this node's pointer will be set to,
        // and which will be used in a compare and swap operation on the
        // predecessor pointer.
        let mut spots : [(*const AtomicPtr<Node<T>>, *mut Node<T>); MAX_HEIGHT];
                spots = [(ptr::null(), ptr::null_mut()); MAX_HEIGHT];

        // This is very similar to the search in get, but we track the
        // predecessors and successors in each lane usin the `spots` variable.
        // We iterate across the list, visting different nodes, and down each
        // node's list of lanes, until we find the point in the lowest lane at
        // which we are to insert our new node.
        'across: while height > inserted {
            'down: for atomic_ptr in lanes {
                let ptr: Ptr<Node<T>> = NonNull::new(atomic_ptr.load(Relaxed));

                match ptr {
                    // If the pointer is null, we are at the end of this lane
                    // and we should move downward.
                    None        => {
                        if height == highest_insert {
                            init_lanes = lanes;
                        }
                        height -= 1;
                        spots[height] = (atomic_ptr, ptr::null_mut());
                        continue 'down;
                    }

                    // If not, we will do a comparison between the element
                    // to be inserted and the element at this node.
                    Some(ptr)   => unsafe {
                        let node: &'a Node<T> = &*ptr.as_ptr();
                        let elem_ref: &T = elem_ptr.as_ref();

                        match elem_ref.cmp(&node.inner.elem) {
                            // If inserted is > 0, that means we have actually
                            // found our own partially inserted node. We can
                            // stop searching.
                            Equal if inserted > 0   => break 'across,

                            // If they are equal, this element has already
                            // been inserted into the list, and we need to
                            // return the element we attempted to insert. The
                            // logic for this depends on whether or not we've
                            // already allocated a node (in a previous
                            // iteration of the 'retry loop). If we have, we
                            // must deallocate that node to avoid leaking it.
                            Equal                   => match &mut new_node {
                                Some(new_node)  => return Some(new_node.as_mut().dealloc()),
                                None            => return Some(ManuallyDrop::take(&mut elem)),
                            }

                            // If the element to be inserted is less than the
                            // element in this node, we want to move down the
                            // lanes.
                            Less                    => {
                                if height == highest_insert {
                                    init_lanes = lanes;
                                }
                                height -= 1;
                                spots[height] = (atomic_ptr, ptr.as_ptr());
                                continue 'down;
                            }

                            // If the element to be inserted is greater than
                            // the element in this node, we want to move across
                            // the list, iterating through the lanes in that
                            // node.
                            Greater                 => {
                                lanes = &node.lanes()[(node.height() - height)..];
                                continue 'across;
                            }
                        }
                    }
                }
            }
        }

        // Allocate the new node if it hasn't already been allocated.
        let new_node: NonNull<Node<T>> = match new_node {
            // If the node is not null, its already been allocated and there is
            // no work to be done.
            Some(new_node)  => new_node,

            // Otherwise, allocate the node, move the element into it, and
            // reset the elem_ptr to point into the heap instead of to the old
            // location on the stack.
            None        => {
                let elem = unsafe { ManuallyDrop::take(&mut elem) };
                let height = super::random_height();
                max_height.fetch_max(height as u8, Relaxed);
                let node = Node::alloc(elem, height);
                elem_ptr = unsafe { NonNull::from(&node.as_ref().inner.elem) };
                new_node = Some(node);
                highest_insert = height;
                new_node.unwrap()
            }
        };

        // The insert loop iterates upward from the lowest lane of this node
        // to its highest, attempting to insert it at each point, performing
        // an atomic compare and swap to identify conflicts with concurrent
        // insertions. In the event of a conflict, we attempt the insertion
        // again, beginning from that level.
        let new_node_addr = new_node.as_ptr();
        let new_node_lanes = unsafe { new_node.as_ref().lanes() };

        for (new, &(pred, succ)) in new_node_lanes.iter().rev().zip(&spots).skip(inserted) {
            let pred: &'a AtomicPtr<Node<T>> = unsafe { &*pred };

            new.store(succ, Release);

            match succ == pred.compare_and_swap(succ, new_node_addr, AcqRel) {
                // We successfully inserted the node into at least one lane,
                // we note that for future iterations.
                true    => inserted += 1,

                // We failed to insert the node, so we retry, beginning with
                // the search.
                false   => continue 'retry,
            }
        }

        return None;
    }
}
