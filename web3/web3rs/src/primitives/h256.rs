//! Implements ethereum H256 type.
//!

use std::fmt::Display;

use super::hex::Hex;

pub type H256 = Hex<[u8; 32]>;

impl Display for H256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#066x}", self)
    }
}

impl From<H256> for [u8; 32] {
    fn from(value: H256) -> Self {
        value.0
    }
}

impl From<[u8; 32]> for H256 {
    fn from(value: [u8; 32]) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_from_hex() {
        // with prefixed.
        let value = H256::from_str("0x2a2b").unwrap();

        let mut target = [0x0u8; 32];

        target[30] = 0x2a;
        target[31] = 0x2b;

        assert_eq!(<[u8; 32]>::from(value), target);

        // without prefixed.
        let value = H256::from_str("2a2b").unwrap();

        let mut target = [0x0u8; 32];

        target[30] = 0x2a;
        target[31] = 0x2b;

        assert_eq!(<[u8; 32]>::from(value), target);
    }

    #[test]
    fn test_to_hex() {
        let hash: H256 = "0x2d9b5206c10d2e29dd8513cb1ab0ce308bf924a9f081ff3bc24e8783fd7e0478"
            .parse()
            .unwrap();

        assert_eq!(
            hash.to_string(),
            "0x2d9b5206c10d2e29dd8513cb1ab0ce308bf924a9f081ff3bc24e8783fd7e0478"
        );

        let hash: H256 = "0x00005206c10d2e29dd8513cb1ab0ce308bf924a9f081ff3bc24e8783fd7e0400"
            .parse()
            .unwrap();

        assert_eq!(
            hash.to_string(),
            "0x00005206c10d2e29dd8513cb1ab0ce308bf924a9f081ff3bc24e8783fd7e0400"
        );
    }
}
