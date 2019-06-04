use std::borrow::Borrow;
use std::cmp::Ordering;

// Same requirements as Ord, but the LHS and RHS can be separate types
pub trait AbstractOrd<Rhs> {
    fn cmp(&self, rhs: &Rhs) -> Ordering;
}

impl<T: Ord> AbstractOrd<T> for T {
    fn cmp(&self, rhs: &T) -> Ordering {
        Ord::cmp(self, rhs)
    }
}

#[repr(transparent)]
pub struct QWrapper<Q: ?Sized>(pub Q);

impl<Q: ?Sized> QWrapper<Q> {
    pub fn new(value: &Q) -> &QWrapper<Q> {
        unsafe { std::mem::transmute(value) }
    }
}

impl<T, Q> AbstractOrd<T> for QWrapper<Q> where
    T: Ord + Borrow<Q>,
    Q: Ord + ?Sized,
{
    fn cmp(&self, rhs: &T) -> Ordering {
        Ord::cmp(&self.0, rhs.borrow())
    }
}
