//! Ethereum Improvement Proposal (EIP) implementations

pub mod eip2718;

#[cfg(feature = "eip712")]
#[cfg_attr(docsrs, doc(cfg(feature = "eip712")))]
pub mod eip712;
