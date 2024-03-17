//! opaque handle type of syscall.

use std::any::TypeId;

/// A transparent type pointer that represents any implementation-related asynchronous system type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Handle {
    data: *const (),
    drop: fn(*const ()),
    type_id: TypeId,
}

/// safely: the handle wrap type `T` must implements `Send + Sync`
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Handle {
    pub fn new<T>(value: T) -> Self
    where
        T: Send + Sync + 'static,
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

    /// Returns `T` reference to the inner value if it is of type `T`, or [`None`].
    ///
    /// As required by the `RASI`, there is no way to get a mutable reference to `T`,
    /// so the inner type `T` should implements `Send + Sync` auto traits.
    pub fn downcast<T>(&self) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        if self.type_id == type_id {
            Some(unsafe { &*(self.data as *const T) })
        } else {
            None
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        (self.drop)(self.data);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    use crate::Handle;

    struct Mock(Arc<AtomicUsize>);

    impl Drop for Mock {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn test_handle() {
        let counter: Arc<AtomicUsize> = Default::default();

        let mock = Mock(counter.clone());

        counter.fetch_add(1, Ordering::Relaxed);

        let handle = Handle::new(mock);

        assert_eq!(handle.downcast::<u32>(), None);

        assert!(handle.downcast::<Mock>().is_some());

        assert_eq!(counter.load(Ordering::Relaxed), 1);

        drop(handle);

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }
}
