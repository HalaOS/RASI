//! ethereum i256 integer type backed by [`ethnum`](https://docs.rs/ethnum/latest/ethnum/index.html)
pub use ethnum::I256;
pub use ethnum::U256;

pub mod num_serde_with_prefixed {
    use std::{
        fmt::{self, Display, Formatter},
        num::ParseIntError,
        str::from_utf8,
    };

    use serde::{
        de::{self, Visitor},
        Deserializer, Serialize, Serializer,
    };

    use super::*;

    #[doc(hidden)]
    pub trait Prefixed: Serialize + Sized {
        fn from_str_prefixed(src: &str) -> Result<Self, ParseIntError>;
    }

    impl Prefixed for I256 {
        fn from_str_prefixed(src: &str) -> Result<Self, ParseIntError> {
            Self::from_str_prefixed(src)
        }
    }

    impl Prefixed for U256 {
        fn from_str_prefixed(src: &str) -> Result<Self, ParseIntError> {
            Self::from_str_prefixed(src)
        }
    }

    #[doc(hidden)]
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Prefixed,
        S: Serializer,
    {
        value.serialize(serializer)
    }

    #[doc(hidden)]
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: Prefixed,
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(FormatVisitor(T::from_str_prefixed))
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
