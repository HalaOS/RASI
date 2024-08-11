use crate::{doc, ExpType};

macro_rules! wrapping {
    ($BUint: ident, $BInt: ident, $Digit: ident) => {
        #[doc = doc::wrapping::impl_desc!()]
        impl<const N: usize> $BInt<N> {
            #[doc = doc::wrapping::wrapping_add!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_add(self, rhs: Self) -> Self {
                Self::from_bits(self.bits.wrapping_add(rhs.bits))
            }

            #[doc = doc::wrapping::wrapping_add_unsigned!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_add_unsigned(self, rhs: $BUint<N>) -> Self {
                self.overflowing_add_unsigned(rhs).0
            }

            #[doc = doc::wrapping::wrapping_sub!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_sub(self, rhs: Self) -> Self {
                Self::from_bits(self.bits.wrapping_sub(rhs.bits))
            }

            #[doc = doc::wrapping::wrapping_sub_unsigned!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_sub_unsigned(self, rhs: $BUint<N>) -> Self {
                self.overflowing_sub_unsigned(rhs).0
            }

            #[doc = doc::wrapping::wrapping_mul!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_mul(self, rhs: Self) -> Self {
                Self::from_bits(self.bits.wrapping_mul(rhs.bits))
            }

            #[doc = doc::wrapping::wrapping_div!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_div(self, rhs: Self) -> Self {
                self.overflowing_div(rhs).0
            }

            #[doc = doc::wrapping::wrapping_div_euclid!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_div_euclid(self, rhs: Self) -> Self {
                self.overflowing_div_euclid(rhs).0
            }

            #[doc = doc::wrapping::wrapping_rem!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_rem(self, rhs: Self) -> Self {
                self.overflowing_rem(rhs).0
            }

            #[doc = doc::wrapping::wrapping_rem_euclid!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_rem_euclid(self, rhs: Self) -> Self {
                self.overflowing_rem_euclid(rhs).0
            }

            #[doc = doc::wrapping::wrapping_neg!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_neg(self) -> Self {
                self.overflowing_neg().0
            }

            #[doc = doc::wrapping::wrapping_shl!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_shl(self, rhs: ExpType) -> Self {
                self.overflowing_shl(rhs).0
            }

            #[doc = doc::wrapping::wrapping_shr!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_shr(self, rhs: ExpType) -> Self {
                self.overflowing_shr(rhs).0
            }

            #[doc = doc::wrapping::wrapping_abs!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_abs(self) -> Self {
                self.overflowing_abs().0
            }

            #[doc = doc::wrapping::wrapping_pow!(I)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn wrapping_pow(self, pow: ExpType) -> Self {
                // as wrapping_mul for signed and unsigned is the same
                Self::from_bits(self.bits.wrapping_pow(pow))
            }
        }
    };
}

crate::macro_impl!(wrapping);
