//! Represents ethereum account address with builtin eip55 support

use std::{fmt::Display, str::FromStr};

use serde::{de, Deserialize, Serialize};
use sha3::{Digest, Keccak256};

use super::{
    hex::{Hex, HexError},
    serde::BytesVisitor,
};

/// Represents the ethereum address.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Address(Hex<[u8; 20]>);

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<[u8; 20]> for Address {
    fn from(value: [u8; 20]) -> Self {
        Self(Hex(value))
    }
}

impl Address {
    /// Create zero address.
    pub fn zero_address() -> Self {
        [0; 20].into()
    }
}

impl FromStr for Address {
    type Err = HexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hex: Hex<[u8; 20]> = s.parse()?;

        Ok(Self(hex))
    }
}

impl TryFrom<&str> for Address {
    type Error = HexError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for Address {
    type Error = HexError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl Address {
    pub fn to_checksum_string(&self) -> String {
        let mut data = format!("{:#042x}", self.0);

        let digest: [u8; 32] = Keccak256::new()
            .chain_update(&data.as_bytes()[2..])
            .finalize()
            .into();

        let addr = unsafe { &mut data.as_bytes_mut()[2..] };

        for i in 0..addr.len() {
            let byte = digest[i / 2];
            let nibble = 0xf & if i % 2 == 0 { byte >> 4 } else { byte };
            if nibble >= 8 {
                addr[i] = addr[i].to_ascii_uppercase();
            }
        }

        data
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_checksum_string())
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            self.to_checksum_string().serialize(serializer)
        } else {
            serializer.serialize_newtype_struct("address", &self.0 .0.to_vec())
        }
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let data = String::deserialize(deserializer)?;

            Address::from_str(&data).map_err(serde::de::Error::custom)
        } else {
            let bytes = deserializer.deserialize_newtype_struct("address", BytesVisitor)?;

            if bytes.len() < 20 {
                return Err(de::Error::custom(HexError::InvalidHexLength(bytes.len())));
            }

            let mut buff = [0u8; 20];

            buff.copy_from_slice(&bytes[12..]);

            Ok(Self(Hex(buff)))
        }
    }
}

#[cfg(feature = "wallet")]
impl From<&k256::SecretKey> for Address {
    fn from(value: &k256::SecretKey) -> Self {
        use elliptic_curve::sec1::ToEncodedPoint;

        let value = value.public_key();
        let buff = value.to_encoded_point(false);

        let digest: [u8; 32] = Keccak256::new()
            .chain_update(&buff.as_bytes()[1..])
            .finalize()
            .into();

        let buf: [u8; 20] = digest[12..].try_into().unwrap();

        Self(Hex::from(buf))
    }
}

#[cfg(feature = "wallet")]
impl From<k256::SecretKey> for Address {
    fn from(value: k256::SecretKey) -> Self {
        Address::from(&value)
    }
}

#[cfg(feature = "wallet")]
impl From<&k256::PublicKey> for Address {
    fn from(value: &k256::PublicKey) -> Self {
        use elliptic_curve::sec1::ToEncodedPoint;

        let buff = value.to_encoded_point(false);

        let digest: [u8; 32] = Keccak256::new()
            .chain_update(&buff.as_bytes()[1..])
            .finalize()
            .into();

        let buf: [u8; 20] = digest[12..].try_into().unwrap();

        Self(Hex::from(buf))
    }
}

#[cfg(feature = "wallet")]
impl From<k256::PublicKey> for Address {
    fn from(value: k256::PublicKey) -> Self {
        Address::from(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::Address;

    #[test]
    fn test_address() {
        assert_eq!(
            "0x020A6aef4E458630be6f696E8d23C0958029a47d"
                .parse::<Address>()
                .unwrap()
                .to_checksum_string(),
            "0x020A6aef4E458630be6f696E8d23C0958029a47d"
        );
    }
}
