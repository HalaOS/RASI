pub use reweb3_num::types::{I256, U256};

pub type U8 = u8;
pub type U16 = u16;
pub type U32 = u32;
pub type U64 = u64;
pub type U128 = u128;

pub type I8 = i8;
pub type I16 = i16;
pub type I32 = i32;
pub type I64 = i64;
pub type I128 = i128;

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{abi::from_abi, prelude::Bytes};

    #[test]
    fn test_abi_decode() {
        let hex: Bytes = "0x00000000000000000000000000000000000000000000000000ae09a16fb800f1"
            .parse()
            .unwrap();

        let value: U256 = from_abi(&hex).unwrap();

        assert_eq!(value, U256::from(48987234916368625u128));
    }
}
