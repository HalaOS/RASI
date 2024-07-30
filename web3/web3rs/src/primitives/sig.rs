//! Represents EIP1559 Signature

use std::fmt::Display;

use crate::primitives::hex::Hex;

use super::u256::U256;

#[derive(Debug, Default)]
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
