//! Implements ethereum H256 type.
//!

use sha3::{Digest, Keccak256};

use super::hex::Hex;

/// Represents solidity/jsonrpc type H256.
pub type H256 = Hex<[u8; 32]>;

/// Calculate the input data's keccak256 hash
pub fn keccak256<S>(bytes: S) -> H256
where
    S: AsRef<[u8]>,
{
    let mut hasher = Keccak256::new();

    hasher.update(bytes.as_ref());

    Hex::<[u8; 32]>(hasher.finalize().into())
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
