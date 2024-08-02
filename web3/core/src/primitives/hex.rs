//! Utility to convert u8 slice to and from hexadecimal strings.

use std::{
    fmt::{Display, LowerHex, UpperHex, Write},
    num::ParseIntError,
    str::FromStr,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum HexError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error("Invalid hex length: {0}")]
    InvalidHexLength(usize),
}

/// Represent a ethereum hex string type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hex<T>(pub T);

impl<T> Hex<T>
where
    T: AsRef<[u8]>,
{
    /// Returns the hex bytes length.
    pub fn len(&self) -> usize {
        self.0.as_ref().len()
    }
}

impl<T> LowerHex for Hex<T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buf = self.0.as_ref();

        let mut prefix = String::new();
        let mut leading_spaces = String::new();

        if let Some(width) = f.width() {
            let content_with = buf.len() * 2 + if f.alternate() { 2 } else { 0 };

            if width > content_with {
                if f.sign_aware_zero_pad() {
                    prefix = "0".repeat(width - content_with);
                } else {
                    leading_spaces = " ".repeat(width - content_with)
                }
            }
        }

        let mut content = String::with_capacity(buf.len() * 2);
        for &b in buf {
            write!(&mut content, "{:02x}", b)?;
        }

        if f.alternate() {
            write!(f, "{}0x{}{}", leading_spaces, prefix, content)
        } else {
            write!(f, "{}{}{}", leading_spaces, prefix, content)
        }
    }
}

impl<T> UpperHex for Hex<T>
where
    T: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let buf = self.0.as_ref();

        let mut prefix = String::new();
        let mut leading_spaces = String::new();

        if let Some(width) = f.width() {
            let content_with = buf.len() * 2 + if f.alternate() { 2 } else { 0 };

            if width > content_with {
                if f.sign_aware_zero_pad() {
                    prefix = "0".repeat(width - content_with);
                } else {
                    leading_spaces = " ".repeat(width - content_with)
                }
            }
        }

        let mut content = String::with_capacity(buf.len() * 2);
        for &b in buf {
            write!(&mut content, "{:02X}", b)?;
        }

        if f.alternate() {
            write!(f, "{}0x{}{}", leading_spaces, prefix, content)
        } else {
            write!(f, "{}{}{}", leading_spaces, prefix, content)
        }
    }
}

impl FromStr for Hex<Vec<u8>> {
    type Err = HexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = if s.starts_with("0x") { &s[2..] } else { s };

        if s.len() < 2 {
            if s == "0" {
                return Ok(Hex(vec![0x0]));
            }
        }

        let s = if s.len() % 2 != 0 {
            "0".to_string() + s
        } else {
            s.to_owned()
        };

        let buf: Result<Vec<u8>, ParseIntError> = (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
            .collect();

        Ok(Hex(buf?))
    }
}

impl<const N: usize> FromStr for Hex<[u8; N]> {
    type Err = HexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = if s.starts_with("0x") { &s[2..] } else { s };

        if s.len() > N * 2 {
            return Err(HexError::InvalidHexLength(s.len()));
        }

        if s.len() % 2 != 0 {
            return Err(HexError::InvalidHexLength(s.len()));
        }

        let offset = N - s.len() / 2;

        let mut buf = [0u8; N];

        for i in offset..N {
            buf[i] = u8::from_str_radix(&s[(i - offset) * 2..(i - offset) * 2 + 2], 16)?;
        }

        Ok(Hex(buf))
    }
}

impl TryFrom<&str> for Hex<Vec<u8>> {
    type Error = HexError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for Hex<Vec<u8>> {
    type Error = HexError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<const N: usize> TryFrom<&str> for Hex<[u8; N]> {
    type Error = HexError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl<const N: usize> TryFrom<String> for Hex<[u8; N]> {
    type Error = HexError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Vec<u8>> for Hex<Vec<u8>> {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}
impl From<Hex<Vec<u8>>> for Vec<u8> {
    fn from(value: Hex<Vec<u8>>) -> Self {
        value.0
    }
}

impl<const N: usize> From<[u8; N]> for Hex<[u8; N]> {
    fn from(value: [u8; N]) -> Self {
        Self(value)
    }
}
impl<const N: usize> From<Hex<[u8; N]>> for [u8; N] {
    fn from(value: Hex<[u8; N]>) -> Self {
        value.0
    }
}

impl Serialize for Hex<Vec<u8>> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            format!("{:#x}", self).serialize(serializer)
        } else {
            serializer.serialize_newtype_struct("bytes", &self.0)
        }
    }
}

impl<'de> Deserialize<'de> for Hex<Vec<u8>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let data = String::deserialize(deserializer)?;

            Hex::<Vec<u8>>::from_str(&data).map_err(serde::de::Error::custom)
        } else {
            let hex = Vec::<u8>::deserialize(deserializer)?;

            Ok(Hex(hex))
        }
    }
}

impl<const N: usize> Serialize for Hex<[u8; N]> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            format!("{:#x}", self).serialize(serializer)
        } else {
            serializer.serialize_newtype_struct("bytesN", &self.0.to_vec())
        }
    }
}

impl<'de, const N: usize> Deserialize<'de> for Hex<[u8; N]> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let data = String::deserialize(deserializer)?;

            Hex::<[u8; N]>::from_str(&data).map_err(serde::de::Error::custom)
        } else {
            let buf = Vec::<u8>::deserialize(deserializer)?;

            if buf.len() != N {
                return Err(HexError::InvalidHexLength(buf.len()))
                    .map_err(serde::de::Error::custom);
            }

            let mut hex = [0u8; N];

            hex.copy_from_slice(&buf);

            Ok(Hex(hex))
        }
    }
}

impl<T: AsRef<[u8]>> Display for Hex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#x}", self)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_lower_hex_padding() {
        assert_eq!(
            format!("{:#66x}", Hex(&[0u8, 0x1, 0xa])),
            "                                                          0x00010a"
        );

        assert_eq!(
            format!("{:#066x}", Hex(&[0u8, 0x1, 0xa])),
            "0x000000000000000000000000000000000000000000000000000000000000010a"
        );
    }

    #[test]
    fn test_upper_hex_padding() {
        assert_eq!(
            format!("{:#66X}", Hex(&[0u8, 0x1, 0xa])),
            "                                                          0x00010A"
        );

        assert_eq!(
            format!("{:#066X}", Hex(&[0u8, 0x1, 0xa])),
            "0x000000000000000000000000000000000000000000000000000000000000010A"
        );
    }
}
