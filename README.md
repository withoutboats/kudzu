# Kudzu - concurrent set and map data structures

> Information grows monotonically.

Kudzu provides a `Map` and `Set` implemented on top of a concurrent skiplist.
The key difference between the types in kudzu and other concurrent data
structures is that kudzu's data structures do not support *remove* operations.
This limitation makes the types much simpler to implement and hopefully more
performant, with less coordination overhead, while still being useful for many
applications.

## Use cases

These can be used for any concurrent algorithm in which a map or set only
grows, without ever losing members. For example, this can be combined with
rayon as a memoization table for divide-and-conquer algorithms with repeating
subproblems (e.g. fibonacci).

## Concurrency properties

Assuming my implementation is correct and my reasoning is sound (big
assumptions), all of these things should be true:

- Data races are impossible.
- All operations on the map are **lock-free**: it is not possible for two
  concurrent operations on the same set to create a deadlock.
- Lookup is **wait-free**: looking up an item in the set never waits on another
  thread to complete.
- At no point is the skiplist in a state inconsistent with the properties of a
  skiplist (that is, every lane will always be a superset of the lane below
  it).

Lookup accesses are performed by simply searching through the skiplist using
Relaxed loads, with no locking or coordination.

Inserts are performed by finding the location to insert and performing a CAS on
the pointers in each lane that the node will be inserted to. If the CAS fails
on the lowest lane, the node is not inserted and the entire insertion algorithm
is tried again. However, on any higher lane than the lowest, we simply stop
inserting once an insertion fails. This leaves the list flatter than it would
ideally be in instances of contention, but makes handling that contention much
cheaper than retrying would be - I hope this proves to be a beneficial trade
off.

(Inserting an element which is found to already be present returns the element
you attempted to insert without changing the value already in the set.)

Because of this insertion strategy and the absence of removal operations,
concurrency correctness can be maintained by simply using atomic CAS operations
on each modified pointer, rather than having to track additional metadata and
node-level locks.

I owe much to [this paper][paper], which describes a somewhat similar algorithm
and suggests something similar to my algorithm in the conclusion.

## Memory layout optimizations

This skiplist also has a highly optimized memory layout to improve performance.
Each node contains all of its lanes inline with the element, but is also
precisely sized so that it does not contain space for unused lanes. This means
that the space overhead for each node is only on average 2 pointers and 1 byte
(to track the number of lanes for deallocation purposes).

We also store lanes in reverse order, with the highest lane as the first
element. Between this and making the nodes inline, we should have much better
memory locality, because nodes are most often visited in their highest lane,
not their lowest.
