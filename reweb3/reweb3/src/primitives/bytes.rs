use super::hex::Hex;

pub type Bytes = Hex<Vec<u8>>;

impl From<Bytes> for bytes::Bytes {
    fn from(value: Bytes) -> Self {
        Self::from(value.0)
    }
}

macro_rules! define_bytes_n {
    ($ident: ident, $len: expr) => {
        #[doc = concat!("Represents solidity type ",stringify!($ident))]
        pub type $ident = Hex<[u8; $len]>;
    };
    ($ident: ident, $len: expr, $($ident_next: ident, $len_next: expr),+) => {
        define_bytes_n!($ident,$len);
        define_bytes_n!($($ident_next, $len_next),+);
    };
}

define_bytes_n!(
    Bytes1, 1, Bytes2, 2, Byte3, 3, Bytes4, 4, Bytes5, 5, Bytes6, 6, Bytes7, 7, Byte8, 8, Bytes9,
    9, Bytes10, 10, Bytes11, 11, Bytes12, 12, Byte13, 13, Bytes14, 14, Bytes15, 15, Bytes16, 16,
    Bytes17, 17, Byte18, 18, Bytes19, 19, Bytes20, 20, Bytes21, 21, Bytes22, 22, Byte23, 23,
    Bytes24, 24, Bytes25, 25, Bytes26, 26, Bytes27, 27, Byte28, 28, Bytes29, 29, Bytes30, 30,
    Bytes31, 31, Bytes32, 32
);
