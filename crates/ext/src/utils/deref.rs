use std::ops;

/// This trait extend [`Deref`](ops::Deref) to add function `deref_map`
pub trait DerefExt: ops::Deref {
    fn deref_map<F, R>(self, f: F) -> MapDeref<Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Target) -> &R,
    {
        MapDeref { deref: self, f }
    }
}

/// The type map [`Deref`](ops::Deref) target to another type.
pub struct MapDeref<T, F> {
    deref: T,
    f: F,
}

impl<T, F, R> ops::Deref for MapDeref<T, F>
where
    F: Fn(&T::Target) -> &R,
    T: ops::Deref,
{
    type Target = R;

    fn deref(&self) -> &Self::Target {
        (self.f)(self.deref.deref())
    }
}

/// Implement [`DerefExt`] for all type that implement trait [`Deref`](ops::Deref)
impl<T> DerefExt for T where T: ops::Deref {}
