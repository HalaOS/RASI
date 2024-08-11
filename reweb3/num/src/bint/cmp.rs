use crate::nightly::impl_const;
use core::cmp::{Ord, Ordering, PartialOrd};

macro_rules! cmp {
    ($BUint: ident, $BInt: ident, $Digit: ident) => {
        // impl_const! {
        //     impl<const N: usize> const PartialEq for $BInt<N> {
        //         #[inline]
        //         fn eq(&self, other: &Self) -> bool {
        //             Self::eq(self, other)
        //         }
        //     }
        // }

        // impl<const N: usize> Eq for $BInt<N> {}

        impl_const! {
            impl<const N: usize> const PartialOrd for $BInt<N> {
                #[inline]
                fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                    Some(self.cmp(other))
                }
            }
        }

        impl_const! {
            impl<const N: usize> const Ord for $BInt<N> {
                #[inline]
                fn cmp(&self, other: &Self) -> Ordering {
                    Self::cmp(self, other)
                }

                #[inline]
                fn max(self, other: Self) -> Self {
                    Self::max(self, other)
                }

                #[inline]
                fn min(self, other: Self) -> Self {
                    Self::min(self, other)
                }

                #[inline]
                fn clamp(self, min: Self, max: Self) -> Self {
                    Self::clamp(self, min, max)
                }
            }
        }
    };
}

crate::macro_impl!(cmp);
