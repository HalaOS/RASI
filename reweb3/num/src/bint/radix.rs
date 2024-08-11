use crate::doc;
use crate::errors::ParseIntError;
use crate::int::radix::assert_range;
use alloc::string::String;
use alloc::vec::Vec;
use core::num::IntErrorKind;

macro_rules! radix {
    ($BUint: ident, $BInt: ident, $Digit: ident) => {
        #[doc = doc::radix::impl_desc!($BInt)]
        impl<const N: usize> $BInt<N> {
            /// Converts a byte slice in a given base to an integer. The input slice must contain ascii/utf8 characters in [0-9a-zA-Z].
            ///
            /// This function is equivalent to the [`from_str_radix`](#method.from_str_radix) function for a string slice equivalent to the byte slice and the same radix.
            ///
            /// Returns `None` if the conversion of the byte slice to string slice fails or if a digit is larger than or equal to the given radix, otherwise the integer is wrapped in `Some`.
            #[inline]
            pub const fn parse_bytes(buf: &[u8], radix: u32) -> Option<Self> {
                let s = crate::nightly::option_try!(crate::nightly::ok!(core::str::from_utf8(buf)));
                crate::nightly::ok!(Self::from_str_radix(s, radix))
            }

            /// Converts a slice of big-endian digits in the given radix to an integer. The digits are first converted to an unsigned integer, then this is transmuted to a signed integer. Each `u8` of the slice is interpreted as one digit of base `radix` of the number, so this function will return `None` if any digit is greater than or equal to `radix`, otherwise the integer is wrapped in `Some`.
            ///
            /// For examples, see the
            #[doc = concat!("[`from_radix_be`](crate::", stringify!($BUint), "::from_radix_be) method documentation for [`", stringify!($BUint), "`](crate::", stringify!($BUint), ").")]
            #[inline]
            pub const fn from_radix_be(buf: &[u8], radix: u32) -> Option<Self> {
                match $BUint::from_radix_be(buf, radix) { // TODO: use Option::map when stable
                    Some(uint) => Some(Self::from_bits(uint)),
                    None => None,
                }
            }

            /// Converts a slice of big-endian digits in the given radix to an integer. The digits are first converted to an unsigned integer, then this is transmuted to a signed integer. Each `u8` of the slice is interpreted as one digit of base `radix` of the number, so this function will return `None` if any digit is greater than or equal to `radix`, otherwise the integer is wrapped in `Some`.
            ///
            /// For examples, see the
            #[doc = concat!("[`from_radix_le`](crate::", stringify!($BUint), "::from_radix_le) method documentation for [`", stringify!($BUint), "`](crate::", stringify!($BUint), ").")]
            #[inline]
            pub const fn from_radix_le(buf: &[u8], radix: u32) -> Option<Self> {
                match $BUint::from_radix_le(buf, radix) { // TODO: use Option::map when stable
                    Some(uint) => Some(Self::from_bits(uint)),
                    None => None,
                }
            }

            /// Converts a string slice in a given base to an integer.
            ///
            /// The string is expected to be an optional `+` or `-` sign followed by digits. Leading and trailing whitespace represent an error. Digits are a subset of these characters, depending on `radix`:
            ///
            /// - `0-9`
            /// - `a-z`
            /// - `A-Z`
            ///
            /// # Panics
            ///
            /// This function panics if `radix` is not in the range from 2 to 36 inclusive.
            ///
            /// For examples, see the
            #[doc = concat!("[`from_str_radix`](crate::", stringify!($BUint), "::from_str_radix) method documentation for [`", stringify!($BUint), "`](crate::", stringify!($BUint), ").")]
            #[inline]
            pub const fn from_str_radix(src: &str, radix: u32) -> Result<Self, ParseIntError> {
                assert_range!(radix, 36);
                if src.is_empty() {
                    return Err(ParseIntError {
                        kind: IntErrorKind::Empty,
                    });
                }
                let mut negative = false;
                let mut leading_sign = false;
                let buf = src.as_bytes();
                if buf[0] == b'-' {
                    negative = true;
                    leading_sign = true;
                } else if buf[0] == b'+' {
                    leading_sign = true;
                }

                match $BUint::from_buf_radix_internal::<true, true>(buf, radix, leading_sign) {
                    Ok(uint) => {
                        if negative {
                            if uint.bit(Self::BITS - 1) && uint.trailing_zeros() != Self::BITS - 1 {
                                Err(ParseIntError {
                                    kind: IntErrorKind::NegOverflow,
                                })
                            } else {
                                Ok(Self::from_bits(uint).wrapping_neg())
                            }
                        } else {
                            let out = Self::from_bits(uint);
                            if out.is_negative() {
                                Err(ParseIntError {
                                    kind: IntErrorKind::PosOverflow,
                                })
                            } else {
                                Ok(out)
                            }
                        }
                    }
                    Err(err) => {
                        if let IntErrorKind::PosOverflow = err.kind() {
                            if negative {
                                return Err(ParseIntError {
                                    kind: IntErrorKind::NegOverflow,
                                });
                            }
                        }
                        return Err(err)
                    }
                }
            }

            #[doc = doc::radix::parse_str_radix!($BUint)]
            #[inline]
            pub const fn parse_str_radix(src: &str, radix: u32) -> Self {
                match Self::from_str_radix(src, radix) {
                    Ok(n) => n,
                    Err(e) => panic!("{}", e.description()),
                }
            }

            /// Returns the integer as a string in the given radix.
            ///
            /// # Panics
            ///
            /// This function panics if `radix` is not in the range from 2 to 36 inclusive.
            ///
            /// For examples, see the
            #[doc = concat!("[`to_str_radix`](crate::", stringify!($BUint), "::to_str_radix) method documentation for [`", stringify!($BUint), "`](crate::", stringify!($BUint), ").")]
            #[inline]
            pub fn to_str_radix(&self, radix: u32) -> String {
                if self.is_negative() {
                    format!("-{}", self.unsigned_abs().to_str_radix(radix))
                } else {
                    self.bits.to_str_radix(radix)
                }
            }

            /// Returns the integer's underlying representation as an unsigned integer in the given base in big-endian digit order.
            ///
            /// # Panics
            ///
            /// This function panics if `radix` is not in the range from 2 to 256 inclusive.
            ///
            /// For examples, see the
            #[doc = concat!("[`to_radix_be`](crate::", stringify!($BUint), "::to_radix_be) method documentation for [`", stringify!($BUint), "`]")]
            #[inline]
            pub fn to_radix_be(&self, radix: u32) -> Vec<u8> {
                self.bits.to_radix_be(radix)
            }

            /// Returns the integer's underlying representation as an unsigned integer in the given base in little-endian digit order.
            ///
            /// # Panics
            ///
            /// This function panics if `radix` is not in the range from 2 to 256 inclusive.
            ///
            /// For examples, see the
            #[doc = concat!("[`to_radix_le`](crate::", stringify!($BUint), "::to_radix_le) method documentation for [`", stringify!($BUint), "`](crate::", stringify!($BUint), ").")]
            #[inline]
            pub fn to_radix_le(&self, radix: u32) -> Vec<u8> {
                self.bits.to_radix_le(radix)
            }
        }

    };
}

crate::macro_impl!(radix);
