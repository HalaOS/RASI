//! Represents EIP1559 Signature

use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::primitives::hex::Hex;

use super::{Bytes, HexError, U256};

/// Represents a [`eip1559`](https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1559.md) signature data.
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

        write!(f, "{:#x}", Hex(buff))
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

#[cfg(feature = "wallet")]
#[cfg_attr(docsrs, doc(cfg(feature = "wallet")))]
mod wallet {
    use crate::errors::Error;

    use super::*;

    use ecdsa::RecoveryId;
    use k256::ecdsa::Signature;

    /// Convert tuple ([`Signature`], [`RecoveryId`]) to [`Eip1559Signature`]

    impl From<(Signature, RecoveryId)> for Eip1559Signature {
        fn from(value: (Signature, RecoveryId)) -> Self {
            Self {
                v: value.1.to_byte(),
                r: U256::from_be_bytes(value.0.r().to_bytes().try_into().unwrap()),
                s: U256::from_be_bytes(value.0.s().to_bytes().try_into().unwrap()),
            }
        }
    }

    #[cfg(feature = "wallet")]
    impl TryFrom<Eip1559Signature> for (Signature, RecoveryId) {
        type Error = Error;
        fn try_from(sig: Eip1559Signature) -> Result<(Signature, RecoveryId), Self::Error> {
            let mut buff = [0u8; 64];

            buff[..32].copy_from_slice(&sig.r.to_be_bytes());
            buff[32..].copy_from_slice(&sig.s.to_be_bytes());

            let recover_id = sig.v;

            let sig = Signature::try_from(buff.as_slice())?;

            let recover_id =
                RecoveryId::from_byte(recover_id).ok_or(Error::InvalidRecoveryId(recover_id))?;

            Ok((sig, recover_id))
        }
    }
}
