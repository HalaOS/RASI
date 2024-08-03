//! Type aliases for big signed and unsigned integers. Each is an alias for either a [`BUint`] or a [`BInt`].

use crate::{BInt, BUint};

macro_rules! int_type_doc {
    ($bits: literal, $sign: literal) => {
        concat!($bits, "-bit ", $sign, " integer type.")
    };
}

macro_rules! int_types {
    { $($bits: literal $u: ident $i: ident; ) *}  => {
        $(
            #[doc = int_type_doc!($bits, "unsigned")]
            pub type $u = BUint::<{$bits / 64}>;

            #[doc = int_type_doc!($bits, "signed")]
            pub type $i = BInt::<{$bits / 64}>;
        )*
    };
}

macro_rules! call_types_macro {
    ($name: ident) => {
        $name! {
            128 U128 I128;
            256 U256 I256;
            512 U512 I512;
            1024 U1024 I1024;
            2048 U2048 I2048;
            4096 U4096 I4096;
            8192 U8192 I8192;
        }
    };
}

call_types_macro!(int_types);

impl U256 {
    pub fn to_be_bytes(self) -> [u8; 32] {
        let buf = self.to_radix_be(256);

        let mut target = [0u8; 32];

        target[(32 - buf.len())..].copy_from_slice(&buf);

        target
    }

    pub fn from_be_bytes(buf: [u8; 32]) -> Self {
        Self::from_radix_be(&buf, 256).unwrap()
    }
}

impl I256 {
    pub fn to_be_bytes(self) -> [u8; 32] {
        let buf = self.to_radix_be(256);

        let mut target = [0xffu8; 32];

        target[(32 - buf.len())..].copy_from_slice(&buf);

        target
    }

    pub fn from_be_bytes(buf: [u8; 32]) -> Self {
        Self::from_radix_be(&buf, 256).unwrap()
    }
}

#[cfg(feature = "serde")]
pub mod reweb3_serde {
    use super::*;

    use serde::{de, Deserialize, Serialize};

    impl Serialize for U256 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if serializer.is_human_readable() {
                serializer.serialize_str(self.to_str_radix(16).as_str())
            } else {
                let buf = self.to_be_bytes()[(self.leading_zeros() / 8) as usize..].to_vec();

                serializer.serialize_newtype_struct("uint256", &buf)
            }
        }
    }

    impl<'a> Deserialize<'a> for U256 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'a>,
        {
            if deserializer.is_human_readable() {
                deserializer.deserialize_any(U256Visitor)
            } else {
                let buff = deserializer.deserialize_newtype_struct("uint256", BytesVisitor)?;

                Self::from_radix_be(&buff, 256).ok_or(de::Error::custom("u256: Out of range"))
            }
        }
    }

    impl Serialize for I256 {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if serializer.is_human_readable() {
                serializer.serialize_str(self.to_str_radix(16).as_str())
            } else {
                let buf = self.to_be_bytes()[(self.leading_ones() / 8 - 1) as usize..].to_vec();
                serializer.serialize_newtype_struct("int256", &buf)
            }
        }
    }

    impl<'a> Deserialize<'a> for I256 {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'a>,
        {
            if deserializer.is_human_readable() {
                deserializer.deserialize_any(I256Visitor)
            } else {
                let buff = deserializer.deserialize_newtype_struct("int256", BytesVisitor)?;

                Self::from_radix_be(&buff, 256).ok_or(de::Error::custom("i256: Out of range"))
            }
        }
    }

    struct BytesVisitor;

    impl<'de> de::Visitor<'de> for BytesVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "expect bytes/bytes<M>")
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(v)
        }
    }

    struct U256Visitor;

    impl<'de> de::Visitor<'de> for U256Visitor {
        type Value = U256;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "expect string/number")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.starts_with("0x") {
                U256::from_str_radix(&v[2..], 16).map_err(de::Error::custom)
            } else {
                U256::from_str_radix(v, 10).map_err(de::Error::custom)
            }
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_u128(v as u128)
        }

        fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(U256::from(v))
        }
    }

    struct I256Visitor;

    impl<'de> de::Visitor<'de> for I256Visitor {
        type Value = I256;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "expect string/number")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.starts_with("0x") {
                I256::from_str_radix(&v[2..], 16).map_err(de::Error::custom)
            } else if v.starts_with("-0x") {
                I256::from_str_radix(&v[3..], 16).map_err(de::Error::custom)
            } else {
                I256::from_str_radix(v, 10).map_err(de::Error::custom)
            }
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_i128(v as i128)
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_i128(v as i128)
        }

        fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(I256::from(v))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_int_bits {
        { $($bits: literal $u: ident $i: ident; ) *} => {
            $(
                assert_eq!($u::BITS, $bits);
                assert_eq!($i::BITS, $bits);
            )*
        }
    }

    #[test]
    fn test_int_bits() {
        call_types_macro!(assert_int_bits);
    }
}
