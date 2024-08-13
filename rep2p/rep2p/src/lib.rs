pub mod keystore;
pub mod multiaddr;
pub mod routetable;
pub mod serve;
pub mod transport;

mod switch;
pub use switch::*;

mod errors;
pub use errors::*;

pub use identity;

#[allow(renamed_and_removed_lints)]
mod proto;

macro_rules! driver_wrapper {
    ([$doc:expr]$ident:ident[$driver: path]) => {
        #[doc = $doc]
        pub struct $ident(Box<dyn $driver>);

        unsafe impl Send for $ident {}

        impl<D: $driver + 'static> From<D> for $ident {
            fn from(value: D) -> Self {
                Self(Box::new(value))
            }
        }

        impl std::ops::Deref for $ident {
            type Target = dyn $driver;
            fn deref(&self) -> &Self::Target {
                &*self.0
            }
        }

        impl std::ops::DerefMut for $ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut *self.0
            }
        }

        impl $ident {
            pub fn as_driver(&mut self) -> &mut dyn $driver {
                &mut *self.0
            }
        }
    };
}

pub(crate) use driver_wrapper;