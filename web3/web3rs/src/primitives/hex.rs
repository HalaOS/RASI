//! Utility to convert u8 slice to and from hexadecimal strings.

use std::{
    fmt::{LowerHex, UpperHex, Write},
    num::ParseIntError,
    str::FromStr,
};

#[derive(Debug, thiserror::Error)]
pub enum HexError {
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error("Invalid hex length: {0}")]
    InvalidHexLength(usize),
}

/// Represent a ethereum hex string type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hex<T>(pub T);

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
