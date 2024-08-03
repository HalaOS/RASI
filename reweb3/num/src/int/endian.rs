#[cfg(test)]
macro_rules! tests {
    ($Digit: ident; $int: ty) => {
        test_bignum! {
            function: <$int>::from_be(a: $int)
        }
        test_bignum! {
            function: <$int>::from_le(a: $int)
        }
        test_bignum! {
            function: <$int>::to_be(a: $int)
        }
        test_bignum! {
            function: <$int>::to_le(a: $int)
        }
    };
}

#[cfg(test)]
pub(crate) use tests;
