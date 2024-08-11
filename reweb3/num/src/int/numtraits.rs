macro_rules! as_primitive_impl {
    ($Int: ident; $($ty: ty), *) => {
        $(
            impl<const N: usize> AsPrimitive<$ty> for $Int<N> {
                #[inline]
                fn as_(self) -> $ty {
                    <$ty as crate::cast::CastFrom<Self>>::cast_from(self)
                }
            }
        )*
    }
}

pub(crate) use as_primitive_impl;

macro_rules! num_trait_impl {
    ($Int: ident, $tr: ident, $method: ident, $ret: ty) => {
        //crate::nightly::impl_const! {
        impl<const N: usize> $tr for $Int<N> {
            #[inline]
            fn $method(&self, rhs: &Self) -> $ret {
                Self::$method(*self, *rhs)
            }
        }
        //}
    };
}

pub(crate) use num_trait_impl;

macro_rules! as_bigint_impl {
    ([$($ty: ty), *] as $Big: ident) => {
        $(
            //crate::nightly::impl_const! {
                impl<const N: usize> AsPrimitive<$Big<N>> for $ty {
                    #[inline]
                    fn as_(self) -> $Big<N> {
                        $Big::cast_from(self)
                    }
                }
            //}
        )*
    }
}

pub(crate) use as_bigint_impl;

macro_rules! impls {
    ($Int: ident, $BUint: ident, $BInt: ident) => {
        //crate::nightly::impl_const! {
            impl<const N: usize> Bounded for $Int<N> {
                #[inline]
                fn min_value() -> Self {
                    Self::MIN
                }

                #[inline]
                fn max_value() -> Self {
                    Self::MAX
                }
            }
        //}

        num_trait_impl!($Int, CheckedAdd, checked_add, Option<Self>);
        num_trait_impl!($Int, CheckedDiv, checked_div, Option<Self>);
        num_trait_impl!($Int, CheckedMul, checked_mul, Option<Self>);
        num_trait_impl!($Int, CheckedRem, checked_rem, Option<Self>);
        num_trait_impl!($Int, CheckedSub, checked_sub, Option<Self>);

        num_trait_impl!($Int, SaturatingAdd, saturating_add, Self);
        num_trait_impl!($Int, SaturatingMul, saturating_mul, Self);
        num_trait_impl!($Int, SaturatingSub, saturating_sub, Self);

        num_trait_impl!($Int, WrappingAdd, wrapping_add, Self);
        num_trait_impl!($Int, WrappingMul, wrapping_mul, Self);
        num_trait_impl!($Int, WrappingSub, wrapping_sub, Self);

        //crate::nightly::impl_const! {
            impl<const N: usize> CheckedNeg for $Int<N> {
                #[inline]
                fn checked_neg(&self) -> Option<Self> {
                    Self::checked_neg(*self)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> CheckedShl for $Int<N> {
                #[inline]
                fn checked_shl(&self, rhs: u32) -> Option<Self> {
                    Self::checked_shl(*self, rhs as ExpType)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> CheckedShr for $Int<N> {
                #[inline]
                fn checked_shr(&self, rhs: u32) -> Option<Self> {
                    Self::checked_shr(*self, rhs as ExpType)
                }
            }
        //}

        impl<const N: usize> CheckedEuclid for $Int<N> {
            #[inline]
            fn checked_div_euclid(&self, rhs: &Self) -> Option<Self> {
                Self::checked_div_euclid(*self, *rhs)
            }

            #[inline]
            fn checked_rem_euclid(&self, rhs: &Self) -> Option<Self> {
                Self::checked_rem_euclid(*self, *rhs)
            }
        }

        impl<const N: usize> Euclid for $Int<N> {
            #[inline]
            fn div_euclid(&self, rhs: &Self) -> Self {
                Self::div_euclid(*self, *rhs)
            }

            #[inline]
            fn rem_euclid(&self, rhs: &Self) -> Self {
                Self::rem_euclid(*self, *rhs)
            }
        }

        //crate::nightly::impl_const! {
            impl<const N: usize> WrappingNeg for $Int<N> {
                #[inline]
                fn wrapping_neg(&self) -> Self {
                    Self::wrapping_neg(*self)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> WrappingShl for $Int<N> {
                #[inline]
                fn wrapping_shl(&self, rhs: u32) -> Self {
                    Self::wrapping_shl(*self, rhs as ExpType)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> WrappingShr for $Int<N> {
                #[inline]
                fn wrapping_shr(&self, rhs: u32) -> Self {
                    Self::wrapping_shr(*self, rhs as ExpType)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> Pow<ExpType> for $Int<N> {
                type Output = Self;

                #[inline]
                fn pow(self, exp: ExpType) -> Self {
                    Self::pow(self, exp)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> Saturating for $Int<N> {
                #[inline]
                fn saturating_add(self, rhs: Self) -> Self {
                    Self::saturating_add(self, rhs)
                }

                #[inline]
                fn saturating_sub(self, rhs: Self) -> Self {
                    Self::saturating_sub(self, rhs)
                }
            }
        //}

        crate::int::numtraits::as_primitive_impl!($Int; u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

        crate::int::numtraits::as_bigint_impl!([u8, u16, u32, usize, u64, u128, i8, i16, i32, isize, i64, i128, char, bool, f32, f64] as $Int);

        //crate::nightly::impl_const! {
            impl<const N: usize, const M: usize> AsPrimitive<$BUint<M>> for $Int<N> {
                #[inline]
                fn as_(self) -> crate::$BUint<M> {
                    crate::$BUint::<M>::cast_from(self)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize, const M: usize> AsPrimitive<$BInt<M>> for $Int<N> {
                #[inline]
                fn as_(self) -> crate::$BInt<M> {
                    crate::$BInt::<M>::cast_from(self)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> MulAdd for $Int<N> {
                type Output = Self;

                #[inline]
                fn mul_add(self, a: Self, b: Self) -> Self {
                    (self * a) + b
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> MulAddAssign for $Int<N> {
                #[inline]
                fn mul_add_assign(&mut self, a: Self, b: Self) {
                    *self = self.mul_add(a, b);
                }
            }
        //}

        impl<const N: usize> Num for $Int<N> {
            type FromStrRadixErr = crate::errors::ParseIntError;

            #[inline]
            fn from_str_radix(string: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
                Self::from_str_radix(string, radix)
            }
        }

        //crate::nightly::impl_const! {
            impl<const N: usize> num_traits::NumCast for $Int<N> {
                fn from<T: ToPrimitive>(_n: T) -> Option<Self> {
                    panic!(concat!(crate::errors::err_prefix!(), "`num_traits::NumCast` trait is not supported for ", stringify!($Int)))
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> One for $Int<N> {
                #[inline]
                fn one() -> Self {
                    Self::ONE
                }

                #[inline]
                fn is_one(&self) -> bool {
                    core::cmp::PartialEq::eq(self, &Self::ONE)
                }
            }
        //}

        //crate::nightly::impl_const! {
            impl<const N: usize> Zero for $Int<N> {
                #[inline]
                fn zero() -> Self {
                    Self::ZERO
                }

                #[inline]
                fn is_zero(&self) -> bool {
                    Self::is_zero(self)
                }
            }
        //}
    }
}

pub(crate) use impls;

macro_rules! prim_int_method {
    { $(fn $name: ident ($($arg: ident $(: $ty: ty)?), *) -> $ret: ty;) * } => {
        $(
            #[inline]
            fn $name($($arg $(: $ty)?), *) -> $ret {
                Self::$name($($arg), *)
            }
        )*
    };
}

pub(crate) use prim_int_method;

macro_rules! prim_int_methods {
    () => {
        crate::int::numtraits::prim_int_method! {
            fn count_ones(self) -> u32;
            fn count_zeros(self) -> u32;
            fn leading_zeros(self) -> u32;
            fn trailing_zeros(self) -> u32;
            fn rotate_left(self, n: u32) -> Self;
            fn rotate_right(self, n: u32) -> Self;
            fn swap_bytes(self) -> Self;
            fn from_be(x: Self) -> Self;
            fn from_le(x: Self) -> Self;
            fn to_be(self) -> Self;
            fn to_le(self) -> Self;
            fn pow(self, exp: u32) -> Self;
        }

        #[cfg(has_leading_trailing_ones)]
        #[inline]
        fn leading_ones(self) -> u32 {
            Self::leading_ones(self)
        }

        #[cfg(has_leading_trailing_ones)]
        #[inline]
        fn trailing_ones(self) -> u32 {
            Self::trailing_ones(self)
        }

        #[cfg(has_reverse_bits)]
        #[inline]
        fn reverse_bits(self) -> Self {
            Self::reverse_bits(self)
        }
    };
}

pub(crate) use prim_int_methods;
