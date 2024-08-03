//! Represents EIP1559 Signature

use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::primitives::hex::Hex;

use super::{Bytes, HexError, U256};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Eip1559Signature {
    pub v: u8,
    pub r: U256,
    pub s: U256,
}

impl Display for Eip1559Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buff = [0u8; 65];

        buff[0] = self.v;

        buff[1..33].copy_from_slice(&self.r.to_be_bytes());
        buff[33..].copy_from_slice(&self.s.to_be_bytes());

        write!(f, "{:067x}", Hex(buff))
    }
}

impl FromStr for Eip1559Signature {
    type Err = HexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let buff = Bytes::from_str(s)?;

        if buff.len() != 65 {
            return Err(HexError::InvalidHexLength(65));
        }

        let buff = buff.0;

        let mut r = [0u8; 32];
        let mut s = [0u8; 32];

        r.copy_from_slice(&buff[1..33]);
        s.copy_from_slice(&buff[33..]);

        Ok(Self {
            v: buff[0],
            r: U256::from_be_bytes(r),
            s: U256::from_be_bytes(s),
        })
    }
}
