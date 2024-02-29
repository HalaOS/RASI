//! opaque handle type of syscall.

use std::any::TypeId;

/// A transparent type pointer that represents any implementation-related asynchronous system type.
pub struct Handle {
    data: *const (),
    drop: fn(*const ()),
    type_id: TypeId,
}

impl Handle {
    pub fn new<T>(value: T) -> Self
    where
        T: Send + Sync + Drop + 'static,
    {
        Self {
            data: Box::into_raw(Box::new(value)) as *const (),
            drop: Self::handle_drop_fn::<T>,
            type_id: TypeId::of::<T>(),
        }
    }

    /// safely drop opaque data object.
    fn handle_drop_fn<T>(data: *const ()) {
        drop(unsafe { Box::from_raw(data as *mut T) })
    }

    /// Returns `T` reference to the inner value if it is of type `T`, or panic.
    ///
    /// As required by the `NASI`, there is no way to get a mutable reference to `T`,
    /// so the system object should implement auto trais `Send + Sync`.
    pub fn downcast<T>(&self) -> &T
    where
        T: Send + Sync + 'static,
    {
        assert_eq!(self.type_id, TypeId::of::<T>());

        unsafe { &*(self.data as *const T) }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        (self.drop)(self.data);
    }
}
