#![feature(manually_drop_take)]
mod ord;
mod skiplist;

pub mod map;
pub mod set;

use ord::{AbstractOrd, QWrapper};
use skiplist::SkipList;

pub mod raw {
    pub use crate::skiplist::SkipList;
}

pub use map::Map;
pub use set::Set;
