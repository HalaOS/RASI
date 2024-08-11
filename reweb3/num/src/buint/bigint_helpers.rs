use crate::digit;
use crate::doc;

macro_rules! bigint_helpers {
    ($BUint: ident, $BInt: ident, $Digit: ident) => {
        impl<const N: usize> $BUint<N> {
            #[doc = doc::bigint_helpers::carrying_add!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn carrying_add(self, rhs: Self, carry: bool) -> (Self, bool) {
                let (s1, o1) = self.overflowing_add(rhs);
                if carry {
                    let (s2, o2) = s1.overflowing_add(Self::ONE);
                    (s2, o1 || o2)
                } else {
                    (s1, o1)
                }
            }

            #[doc = doc::bigint_helpers::borrowing_sub!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn borrowing_sub(self, rhs: Self, borrow: bool) -> (Self, bool) {
                let (s1, o1) = self.overflowing_sub(rhs);
                if borrow {
                    let (s2, o2) = s1.overflowing_sub(Self::ONE);
                    (s2, o1 || o2)
                } else {
                    (s1, o1)
                }
            }

            #[doc = doc::bigint_helpers::widening_mul!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn widening_mul(self, rhs: Self) -> (Self, Self) {
                let mut low = Self::ZERO;
                let mut high = Self::ZERO;
                let mut carry: $Digit;

                let mut i = 0;
                while i < N {
                    carry = 0;
                    let mut j = 0;
                    while j < N - i {
                        let index = i + j;
                        let d = low.digits[index];
                        let (new_digit, new_carry) =
                            digit::$Digit::carrying_mul(self.digits[i], rhs.digits[j], carry, d);
                        carry = new_carry;
                        low.digits[index] = new_digit;
                        j += 1;
                    }
                    while j < N {
                        let index = i + j - N;
                        let d = high.digits[index];
                        let (new_digit, new_carry) =
                            digit::$Digit::carrying_mul(self.digits[i], rhs.digits[j], carry, d);
                        carry = new_carry;
                        high.digits[index] = new_digit;
                        j += 1;
                    }
                    high.digits[i] = carry;
                    i += 1;
                }

                (low, high)
            }

            #[doc = doc::bigint_helpers::carrying_mul!(U)]
            #[must_use = doc::must_use_op!()]
            #[inline]
            pub const fn carrying_mul(self, rhs: Self, carry: Self) -> (Self, Self) {
                let (low, high) = self.widening_mul(rhs);
                let (low, overflow) = low.overflowing_add(carry);
                if overflow {
                    (low, high.wrapping_add(Self::ONE))
                } else {
                    (low, high)
                }
            }
        }
    };
}

crate::macro_impl!(bigint_helpers);
