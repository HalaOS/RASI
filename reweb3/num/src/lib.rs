#![doc = include_str!("../README.md")]

#[macro_use]
extern crate alloc;

mod bint;
mod buint;

pub mod cast;
mod digit;
pub mod doc;
pub mod errors;
mod int;
mod nightly;
pub mod prelude;

pub mod types;

// #[cfg(feature = "nightly")]
// mod float;

// #[cfg(feature = "nightly")]
// pub use float::Float;

#[cfg(test)]
mod test;

#[cfg(test)]
use test::types::*;

type ExpType = u32;

macro_rules! macro_impl {
    ($name: ident) => {
        use crate::bigints::*;

        crate::main_impl!($name);
    };
}

pub(crate) use macro_impl;

macro_rules! main_impl {
    ($name: ident) => {
        $name!(BUint, BInt, u64);
        $name!(BUintD32, BIntD32, u32);
        $name!(BUintD16, BIntD16, u16);
        $name!(BUintD8, BIntD8, u8);
    };
}

use crate::bint::cast::bint_as_different_digit_bigint;
use crate::buint::cast::buint_as_different_digit_bigint;

buint_as_different_digit_bigint!(BUint, BInt, u64; (BUintD32, u32), (BUintD16, u16), (BUintD8, u8));
buint_as_different_digit_bigint!(BUintD32, BIntD32, u32; (BUint, u64), (BUintD16, u16), (BUintD8, u8));
buint_as_different_digit_bigint!(BUintD16, BIntD16, u16; (BUint, u64), (BUintD32, u32), (BUintD8, u8));
buint_as_different_digit_bigint!(BUintD8, BIntD8, u8; (BUint, u64), (BUintD32, u32), (BUintD16, u16));

bint_as_different_digit_bigint!(BUint, BInt, u64; (BIntD32, u32), (BIntD16, u16), (BIntD8, u8));
bint_as_different_digit_bigint!(BUintD32, BIntD32, u32; (BInt, u64), (BIntD16, u16), (BIntD8, u8));
bint_as_different_digit_bigint!(BUintD16, BIntD16, u16; (BInt, u64), (BIntD32, u32), (BIntD8, u8));
bint_as_different_digit_bigint!(BUintD8, BIntD8, u8; (BInt, u64), (BIntD32, u32), (BIntD16, u16));

pub(crate) use main_impl;

mod bigints {
    pub use crate::bint::{BInt, BIntD16, BIntD32, BIntD8};
    pub use crate::buint::{BUint, BUintD16, BUintD32, BUintD8};
}

pub use bigints::*;

/// Trait for fallible conversions between `bnum` integer types.
///
/// Unfortunately, [`TryFrom`] cannot currently be used for conversions between `bnum` integers, since [`TryFrom<T> for T`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html#impl-TryFrom%3CU%3E-for-T) is already implemented by the standard library (and so it is not possible to implement `TryFrom<BUint<M>> for BUint<N>`). When the [`generic_const_exprs`](https://github.com/rust-lang/rust/issues/76560) feature becomes stabilised, it may be possible to use [`TryFrom`] instead of `BTryFrom`. `BTryFrom` is designed to have the same behaviour as [`TryFrom`] for conversions between two primitive types, and conversions between a primitive type and a bnum type. `BTryFrom` is a workaround for the issue described above, and so you should not implement it yourself. It should only be used for conversions between `bnum` integers.
pub trait BTryFrom<T>: Sized {
    type Error;

    fn try_from(from: T) -> Result<Self, Self::Error>;
}
