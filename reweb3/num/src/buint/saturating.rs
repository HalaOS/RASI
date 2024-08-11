use crate::doc;
use crate::ExpType;

macro_rules! saturating {
    ($BUint: ident, $BInt: ident, $Digit: ident) => {
        #[doc = doc::saturating::impl_desc!()]
        impl<const N: usize> $BUint<N> {
            #[inline]
            const fn saturate_up((int, overflow): ($BUint<N>, bool)) -> $BUint<N> {
                if overflow {
                    $BUint::MAX
                } else {
                    int
                }
            }

            #[inline]
            const fn saturate_down((int, overflow): ($BUint<N>, bool)) -> $BUint<N> {
                if overflow {
                    $BUint::MIN
                } else {
                    int
                }
            }

            #[doc = doc::saturating::saturating_add!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn saturating_add(self, rhs: Self) -> Self {
                Self::saturate_up(self.overflowing_add(rhs))
            }

            #[doc = doc::saturating::saturating_add_signed!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn saturating_add_signed(self, rhs: $BInt<N>) -> Self {
                if rhs.is_negative() {
                    Self::saturate_down(self.overflowing_add_signed(rhs))
                } else {
                    Self::saturate_up(self.overflowing_add_signed(rhs))
                }
            }

            #[doc = doc::saturating::saturating_sub!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn saturating_sub(self, rhs: Self) -> Self {
                Self::saturate_down(self.overflowing_sub(rhs))
            }

            #[doc = doc::saturating::saturating_mul!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn saturating_mul(self, rhs: Self) -> Self {
                Self::saturate_up(self.overflowing_mul(rhs))
            }

            #[doc = doc::saturating::saturating_div!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn saturating_div(self, rhs: Self) -> Self {
                self.div_euclid(rhs)
            }

            #[doc = doc::saturating::saturating_pow!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn saturating_pow(self, exp: ExpType) -> Self {
                Self::saturate_up(self.overflowing_pow(exp))
            }
        }
    };
}

crate::macro_impl!(saturating);
