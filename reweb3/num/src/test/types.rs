pub mod big_types {
    macro_rules! big_types_modules {
        ($bits: literal) => {};
    }

    #[cfg(test_int_bits = "64")]
    big_types_modules!(64);

    #[cfg(not(test_int_bits = "64"))]
    big_types_modules!(128);
}

#[cfg(test_int_bits = "64")]
mod small_types {
    #[allow(non_camel_case_types)]
    pub type utest = u64;

    #[allow(non_camel_case_types)]
    pub type itest = i64;
}

#[cfg(not(test_int_bits = "64"))]
mod small_types {}

pub use core::primitive::*;

// #[cfg(test_int_bits = "64")]
// #[allow(non_camel_case_types)]
// pub type ftest = f64;

// #[cfg(not(test_int_bits = "64"))]
// #[allow(non_camel_case_types)]
// pub type ftest = f32;

// #[cfg(feature = "nightly")]
// #[cfg(test_int_bits = "64")]
// pub type FTEST = crate::float::Float<8, 52>;

// #[cfg(feature = "nightly")]
// #[cfg(not(test_int_bits = "64"))]
// pub type FTEST = crate::float::Float<4, 23>;
