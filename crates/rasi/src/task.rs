use std::{future::Future, sync::OnceLock};

use futures::{future::BoxFuture, task::SpawnError};

/// A driver trait for spawn io task.
pub trait SpawnDriver: Sync + Send {
    /// Spawns a task that polls the given future with output () to completion.
    fn spawn(&self, fut: BoxFuture<'static, ()>) -> Result<(), SpawnError>;
}

#[cfg(feature = "futures-spawn")]
mod futures_spawn {
    use futures::{
        future::BoxFuture,
        task::{SpawnError, SpawnExt},
    };

    use super::SpawnDriver;

    pub type ThreadPoolSpawnDriver = futures::executor::ThreadPool;

    impl SpawnDriver for ThreadPoolSpawnDriver {
        fn spawn(&self, fut: BoxFuture<'static, ()>) -> Result<(), SpawnError> {
            SpawnExt::spawn(&self, fut)
        }
    }
}

/// Spawns a task that polls the given future with output `()` to
/// completion.
pub fn spawn_ok<Fut>(fut: Fut)
where
    Fut: Future<Output = ()> + Send + 'static,
{
    get_spawn_driver().spawn(Box::pin(fut)).unwrap()
}

static GLOBAL_SPAWN_DRIVER: OnceLock<Box<dyn SpawnDriver>> = OnceLock::new();

/// Register provided [`SpawnDriver`] as global spawn implementation.
///
/// # Panic
///
/// Multiple calls to this function are not permitted!!!
pub fn register_spawn_driver<T: SpawnDriver + 'static>(driver: T) {
    if GLOBAL_SPAWN_DRIVER.set(Box::new(driver)).is_err() {
        panic!("Multiple calls to register_spawn_driver are not permitted!!!");
    }
}

/// Get the globally registered instance of [`SpawnDriver`].
///
/// If the feature "futures-spawn" is enabled, this function will create a default
/// instance of "SpawnDriver" based on the [`futures::executor::ThreadPool`] implementation,
/// otherwise you should call [`register_ spawn_driver`] to register your own implementation
/// instance.
pub fn get_spawn_driver() -> &'static dyn SpawnDriver {
    #[cfg(feature = "futures-spawn")]
    return GLOBAL_SPAWN_DRIVER
        .get_or_init(|| {
            Box::new(
                futures_spawn::ThreadPoolSpawnDriver::builder()
                    .pool_size(10)
                    .create()
                    .unwrap(),
            )
        })
        .as_ref();

    #[cfg(not(feature = "futures-spawn"))]
    return GLOBAL_SPAWN_DRIVER
        .get()
        .expect("call register_spawn_driver first.")
        .as_ref();
}
