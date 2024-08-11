macro_rules! ilog {
    ($method: ident $(, $base: ident : $ty: ty)?) => {
        #[doc = doc::$method!(I)]
        #[must_use = doc::must_use_op!()]
        #[inline]
        pub const fn $method(self, $($base : $ty),*) -> ExpType {
            $(
                if $base.le(&<$ty>::ONE) {
                    panic!(errors::err_msg!(errors::invalid_log_base!()))
                }
            ), *
            if self.is_negative() {
                panic!(errors::err_msg!(errors::non_positive_log_message!()))
            } else {
                self.bits.$method($($base.bits)?)
            }
        }
    }
}

#[cfg(debug_assertions)]
use crate::errors::option_expect;

use crate::digit;
use crate::ExpType;
use crate::{doc, errors};

use core::default::Default;

use core::iter::{Iterator, Product, Sum};

macro_rules! mod_impl {
    ($BUint: ident, $BInt: ident, $Digit: ident) => {
        /// Big signed integer type, of fixed size which must be known at compile time. Stored as a
        #[doc = concat!(" [`", stringify!($BUint), "`].")]
        ///
        /// Digits of the underlying
        #[doc = concat!("[`", stringify!($BUint), "`](crate::", stringify!($BUint), ")")]
        /// are stored in little endian (least significant digit first). This integer type aims to exactly replicate the behaviours of Rust's built-in signed integer types: [`i8`], [`i16`], [`i32`], [`i64`], [`i128`] and [`isize`]. The const generic parameter `N` is the number of digits that are stored in the underlying
        #[doc = concat!("[`", stringify!($BUint), "`].")]
        ///
        #[doc = doc::arithmetic_doc!($BInt)]

        #[derive(Clone, Copy, Hash, PartialEq, Eq)]
        #[repr(transparent)]
        pub struct $BInt<const N: usize> {
            pub(crate) bits: $BUint<N>,
        }

        impl<const N: usize> $BInt<N> {
            #[doc = doc::count_ones!(I 256)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn count_ones(self) -> ExpType {
                self.bits.count_ones()
            }

            #[doc = doc::count_zeros!(I 256)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn count_zeros(self) -> ExpType {
                self.bits.count_zeros()
            }

            #[doc = doc::leading_zeros!(I 256)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn leading_zeros(self) -> ExpType {
                self.bits.leading_zeros()
            }

            #[doc = doc::trailing_zeros!(I 256)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn trailing_zeros(self) -> ExpType {
                self.bits.trailing_zeros()
            }

            #[doc = doc::leading_ones!(I 256, NEG_ONE)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn leading_ones(self) -> ExpType {
                self.bits.leading_ones()
            }

            #[doc = doc::trailing_ones!(I 256)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn trailing_ones(self) -> ExpType {
                self.bits.trailing_ones()
            }

            #[doc = doc::rotate_left!(I 256, "i")]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn rotate_left(self, n: ExpType) -> Self {
                Self::from_bits(self.bits.rotate_left(n))
            }

            #[doc = doc::rotate_right!(I 256, "i")]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn rotate_right(self, n: ExpType) -> Self {
                Self::from_bits(self.bits.rotate_right(n))
            }

            #[doc = doc::swap_bytes!(I 256, "i")]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn swap_bytes(self) -> Self {
                Self::from_bits(self.bits.swap_bytes())
            }

            #[doc = doc::reverse_bits!(I 256, "i")]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn reverse_bits(self) -> Self {
                Self::from_bits(self.bits.reverse_bits())
            }

            #[doc = doc::unsigned_abs!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn unsigned_abs(self) -> $BUint<N> {
                if self.is_negative() {
                    self.wrapping_neg().bits
                } else {
                    self.bits
                }
            }

            #[doc = doc::pow!(I 256)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn pow(self, exp: ExpType) -> Self {
                #[cfg(debug_assertions)]
                return option_expect!(self.checked_pow(exp), errors::err_msg!("attempt to calculate power with overflow"));

                #[cfg(not(debug_assertions))]
                self.wrapping_pow(exp)
            }

            #[doc = doc::div_euclid!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn div_euclid(self, rhs: Self) -> Self {
                assert!(self.ne(&Self::MIN) || rhs.ne(&Self::NEG_ONE), errors::err_msg!("attempt to divide with overflow"));
                self.wrapping_div_euclid(rhs)
            }

            #[doc = doc::rem_euclid!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn rem_euclid(self, rhs: Self) -> Self {
                assert!(self.ne(&Self::MIN) || rhs.ne(&Self::NEG_ONE), errors::err_msg!("attempt to calculate remainder with overflow"));
                self.wrapping_rem_euclid(rhs)
            }

            #[doc = doc::abs!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn abs(self) -> Self {
                #[cfg(debug_assertions)]
                return option_expect!(self.checked_abs(), errors::err_msg!("attempt to negate with overflow"));

                #[cfg(not(debug_assertions))]
                match self.checked_abs() {
                    Some(int) => int,
                    None => Self::MIN,
                }
            }

            #[doc = doc::signum!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn signum(self) -> Self {
                if self.is_negative() {
                    Self::NEG_ONE
                } else if self.is_zero() {
                    Self::ZERO
                } else {
                    Self::ONE
                }
            }

            #[doc = doc::is_positive!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn is_positive(self) -> bool {
                let signed_digit = self.signed_digit();
                signed_digit.is_positive() || (signed_digit == 0 && !self.bits.is_zero())
            }

            #[doc = doc::is_negative!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn is_negative(self) -> bool {
                self.signed_digit().is_negative()
            }

            #[doc = doc::doc_comment! {
                I 256,
                "Returns `true` if and only if `self == 2^k` for some integer `k`.",

                "let n = " stringify!(I256) "::from(1i16 << 12);\n"
                "assert!(n.is_power_of_two());\n"
                "let m = " stringify!(I256) "::from(90i8);\n"
                "assert!(!m.is_power_of_two());\n"
                "assert!(!((-n).is_power_of_two()));"
            }]
            #[must_use]
            #[inline]
            pub const fn is_power_of_two(self) -> bool {
                !self.is_negative() &&self.bits.is_power_of_two()
            }

            ilog!(ilog, base: Self);
            ilog!(ilog2);
            ilog!(ilog10);

            #[doc = doc::abs_diff!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn abs_diff(self, other: Self) -> $BUint<N> {
                if self.lt(&other) {
                    other.wrapping_sub(self).to_bits()
                } else {
                    self.wrapping_sub(other).to_bits()
                }
            }

            #[doc = doc::next_multiple_of!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn next_multiple_of(self, rhs: Self) -> Self {
                let rem = self.wrapping_rem_euclid(rhs);
                if rem.is_zero() {
                    return self;
                }
                if rem.is_negative() == rhs.is_negative() {
                    self.add(rhs.sub(rem))
                } else {
                    self.sub(rem)
                }
            }

            #[doc = doc::div_floor!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn div_floor(self, rhs: Self) -> Self {
                if rhs.is_zero() {
                    errors::div_zero!();
                }
                let (div, rem) = self.div_rem_unchecked(rhs);
                if rem.is_zero() || self.is_negative() == rhs.is_negative() {
                    div
                } else {
                    div.sub(Self::ONE)
                }
            }

            #[doc = doc::div_ceil!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn div_ceil(self, rhs: Self) -> Self {
                if rhs.is_zero() {
                    errors::div_zero!();
                }
                let (div, rem) = self.div_rem_unchecked(rhs);
                if rem.is_zero() || self.is_negative() != rhs.is_negative() {
                    div
                } else {
                    div.add(Self::ONE)
                }
            }
        }

        impl<const N: usize> $BInt<N> {
            #[doc = doc::bits!(I 256)]
            #[must_use]
            #[inline]
            pub const fn bits(&self) -> ExpType {
                self.bits.bits()
            }

            #[doc = doc::bit!(I 256)]
            #[must_use]
            #[inline]
            pub const fn bit(&self, b: ExpType) -> bool {
                self.bits.bit(b)
            }

            #[inline(always)]
            pub(crate) const fn signed_digit(&self) -> digit::$Digit::SignedDigit {
                self.bits.digits[N - 1] as _
            }

            #[doc = doc::is_zero!(I 256)]
            #[must_use]
            #[inline]
            pub const fn is_zero(&self) -> bool {
                self.bits.is_zero()
            }

            #[doc = doc::is_one!(I 256)]
            #[must_use]
            #[inline]
            pub const fn is_one(&self) -> bool {
                self.bits.is_one()
            }

            /// Creates a signed integer with `bits` as its underlying representation in two's complement.
            ///
            /// This method is faster for casting from a
            #[doc = concat!("[`", stringify!($BUint), "`]")]
            /// to a
            #[doc = concat!("[`", stringify!($BInt), "`]")]
            /// of the same size than using the `As` trait.
            #[must_use]
            #[inline(always)]
            pub const fn from_bits(bits: $BUint<N>) -> Self {
                Self { bits }
            }

            /// This simply returns the underlying representation of the integer in two's complement, as an unsigned integer.
            ///
            /// This method is faster for casting from a
            #[doc = concat!("[`", stringify!($BInt), "`]")]
            /// to a
            #[doc = concat!("[`", stringify!($BUint), "`]")]
            /// of the same size than using the `As` trait.
            #[must_use]
            #[inline(always)]
            pub const fn to_bits(self) -> $BUint<N> {
                self.bits
            }
        }

        impl<const N: usize> Default for $BInt<N> {
            #[doc = doc::default!()]
            #[inline]
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl<const N: usize> Product<Self> for $BInt<N> {
            #[inline]
            fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::ONE, |a, b| a * b)
            }
        }

        impl<'a, const N: usize> Product<&'a Self> for $BInt<N> {
            #[inline]
            fn product<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                iter.fold(Self::ONE, |a, b| a * b)
            }
        }

        impl<const N: usize> Sum<Self> for $BInt<N> {
            #[inline]
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold(Self::ZERO, |a, b| a + b)
            }
        }

        impl<'a, const N: usize> Sum<&'a Self> for $BInt<N> {
            #[inline]
            fn sum<I: Iterator<Item = &'a Self>>(iter: I) -> Self {
                iter.fold(Self::ZERO, |a, b| a + b)
            }
        }

        #[cfg(any(test))]
        impl<const N: usize> quickcheck::Arbitrary for $BInt<N> {
            #[inline]
            fn arbitrary(g: &mut quickcheck::Gen) -> Self {
                Self::from_bits(<$BUint::<N> as quickcheck::Arbitrary>::arbitrary(g))
            }
        }
    };
}

crate::macro_impl!(mod_impl);

pub mod cast;
mod checked;
mod cmp;
mod const_trait_fillers;
mod consts;
mod convert;
mod endian;
mod fmt;
#[cfg(feature = "numtraits")]
mod numtraits;
mod ops;
mod overflowing;
mod radix;
mod saturating;
mod unchecked;
mod wrapping;
