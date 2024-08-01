use serde::{Deserialize, Serialize};

use super::u256::U256;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct U64(U256);

impl From<u64> for U64 {
    fn from(value: u64) -> Self {
        U64(U256::from(value))
    }
}

impl From<U64> for u64 {
    fn from(value: U64) -> Self {
        value.0.as_u64()
    }
}

impl Serialize for U64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for U64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(U64(U256::deserialize(deserializer)?))
    }
}
