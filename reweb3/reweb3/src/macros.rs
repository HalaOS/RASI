pub use reweb3_macros;

#[macro_export]
macro_rules! hardhat_artifact {
    ($abi_path:tt) => {
        $crate::macros::reweb3_macros::contract!($abi_path);
    };
}

#[macro_export]
macro_rules! contract {
    ($ident: ident, $abi_path: expr) => {
        $crate::macros::reweb3_macros::contract!(stringify!($ident), $abi_path);
    };
}
