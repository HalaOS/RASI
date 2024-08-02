use serde::de;

pub mod u_i_256 {

    use core::fmt;
    use std::{
        fmt::{Display, Formatter},
        num::ParseIntError,
        str::from_utf8,
    };

    use serde::{
        de::{self, Visitor},
        Deserializer, Serialize, Serializer,
    };

    use ethnum::{I256, U256};

    use super::BytesVisitor;

    #[doc(hidden)]
    pub trait Fixed32: Serialize + Sized {
        fn from_be_bytes(buf: Vec<u8>) -> Self;

        fn to_vec(&self) -> Vec<u8>;

        fn name() -> &'static str;

        fn from_str_prefixed(src: &str) -> Result<Self, ParseIntError>;
    }

    impl Fixed32 for I256 {
        fn name() -> &'static str {
            "int256"
        }
        fn from_be_bytes(buf: Vec<u8>) -> Self {
            let mut bytes = [0u8; 32];

            bytes.copy_from_slice(&buf);

            Self::from_be_bytes(bytes)
        }

        fn to_vec(&self) -> Vec<u8> {
            self.to_be_bytes().to_vec()
        }

        fn from_str_prefixed(src: &str) -> Result<Self, ParseIntError> {
            Self::from_str_prefixed(src)
        }
    }

    impl Fixed32 for U256 {
        fn name() -> &'static str {
            "uint256"
        }

        fn from_be_bytes(buf: Vec<u8>) -> Self {
            let mut bytes = [0u8; 32];

            bytes.copy_from_slice(&buf);

            Self::from_be_bytes(bytes)
        }

        fn to_vec(&self) -> Vec<u8> {
            self.to_be_bytes().to_vec()
        }

        fn from_str_prefixed(src: &str) -> Result<Self, ParseIntError> {
            Self::from_str_prefixed(src)
        }
    }

    #[doc(hidden)]
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Fixed32,
        S: Serializer,
    {
        if serializer.is_human_readable() {
            value.serialize(serializer)
        } else {
            serializer.serialize_newtype_struct(T::name(), &value.to_vec())
        }
    }

    #[doc(hidden)]
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: Fixed32,
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_any(FormatVisitor(T::from_str_prefixed))
        } else {
            let buf = deserializer.deserialize_newtype_struct(T::name(), BytesVisitor)?;

            Ok(T::from_be_bytes(buf))
        }
    }

    /// Internal visitor struct implementation to facilitate implementing different
    /// serialization formats.
    struct FormatVisitor<F>(F);

    impl<'de, T, E, F> Visitor<'de> for FormatVisitor<F>
    where
        E: Display,
        F: FnOnce(&str) -> Result<T, E>,
    {
        type Value = T;

        fn expecting(&self, f: &mut Formatter) -> fmt::Result {
            f.write_str("a formatted 256-bit integer")
        }

        fn visit_str<E_>(self, v: &str) -> Result<Self::Value, E_>
        where
            E_: de::Error,
        {
            self.0(v).map_err(de::Error::custom)
        }

        fn visit_bytes<E_>(self, v: &[u8]) -> Result<Self::Value, E_>
        where
            E_: de::Error,
        {
            let string = from_utf8(v)
                .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(v), &self))?;
            self.visit_str(string)
        }

        fn visit_i64<E_>(self, v: i64) -> Result<Self::Value, E_>
        where
            E_: de::Error,
        {
            self.0(v.to_string().as_str()).map_err(de::Error::custom)
        }

        fn visit_u64<E_>(self, v: u64) -> Result<Self::Value, E_>
        where
            E_: de::Error,
        {
            self.0(v.to_string().as_str()).map_err(de::Error::custom)
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct BytesVisitor;

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
