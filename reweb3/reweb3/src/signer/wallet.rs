//! This module convert [`SecretKey List`](k256::SecretKey) into a [`Signer`]

use std::fmt::Debug;

use async_trait::async_trait;
use ecdsa::hazmat::bits2field;
use k256::{Secp256k1, SecretKey};
use serde::Serialize;
use sha2::Sha256;

use k256::ecdsa::hazmat::SignPrimitive;

use crate::{
    eip::{eip2718::TypedTransactionRequest, eip712::TypedData},
    errors::{Error, Result},
    prelude::{Address, Bytes, Eip1559Signature, TryIntoBytes},
    wallet::hdwallet::bip32::DriveKey,
};

use super::Signer;

/// A local wallet wrapper that provider [`Signer`] interface.
pub struct WalletSigner {
    keys: Vec<SecretKey>,
    addresses: Vec<Address>,
}

impl<T: IntoIterator<Item = SecretKey>> From<T> for WalletSigner {
    fn from(value: T) -> Self {
        let keys = value.into_iter().collect::<Vec<_>>();

        let addresses = keys
            .iter()
            .map(|key| Address::from(key))
            .collect::<Vec<_>>();

        Self { keys, addresses }
    }
}

#[async_trait]
impl Signer for WalletSigner {
    /// Signs the transaction request and returns the signed transaction request in [`rlp`](crate::rlp) format.
    async fn sign_transaction<T>(&self, request: T, with_account: Option<&Address>) -> Result<Bytes>
    where
        T: TryInto<TypedTransactionRequest> + Send,
        T::Error: Debug + Send,
    {
        let request: TypedTransactionRequest = request
            .try_into()
            .map_err(|err| Error::Other(format!("{:?}", err)))?;

        let hashed = request.sign_hash()?;

        let secret_key = if let Some(address) = with_account {
            self.get_secret_key(address)
                .ok_or(Error::SignerAccount(address.to_owned()))?
        } else {
            self.keys.first().unwrap()
        };

        let z = bits2field::<Secp256k1>(hashed.as_ref())
            .map_err(|err| Error::Other(format!("Convert bits to field error, {}", err)))?;

        let (sig, recid) = secret_key
            .to_nonzero_scalar()
            .try_sign_prehashed_rfc6979::<Sha256>(&z, b"")
            .map_err(|err| Error::Other(format!("sign rfc6979 error, {}", err)))?;

        let recid = recid.unwrap();

        let sig: Eip1559Signature = (sig, recid).into();

        request.rlp_signed(sig)
    }

    /// Calculate the signature of the [`TypedData`] defined in [`eip-712`](https://eips.ethereum.org/EIPS/eip-712)
    async fn sign_typed_data<V>(
        &self,
        typed_data: TypedData<V>,
        with_account: Option<&Address>,
    ) -> Result<Eip1559Signature>
    where
        V: Serialize + Send,
    {
        let secret_key = if let Some(address) = with_account {
            self.get_secret_key(address)
                .ok_or(Error::SignerAccount(address.to_owned()))?
        } else {
            self.keys.first().unwrap()
        };

        let hashed = typed_data.sign_hash()?;

        let z = bits2field::<Secp256k1>(hashed.as_ref())
            .map_err(|err| Error::Other(format!("Convert bits to field error, {}", err)))?;

        let (sig, recid) = secret_key
            .to_nonzero_scalar()
            .try_sign_prehashed_rfc6979::<Sha256>(&z, b"")
            .map_err(|err| Error::Other(format!("sign rfc6979 error, {}", err)))?;

        let recid = recid.unwrap();

        Ok((sig, recid).into())
    }

    /// Returns the signer's account addresss.
    async fn signer_accounts(&self) -> Result<Vec<Address>> {
        Ok(self.addresses.to_owned())
    }
}

impl WalletSigner {
    fn get_secret_key(&self, address: &Address) -> Option<&SecretKey> {
        self.addresses
            .iter()
            .enumerate()
            .find(|(_, addr)| *addr == address)
            .map(|(index, _)| &self.keys[index])
    }
}

impl WalletSigner {
    /// Create signer with hex str of one private key.
    pub fn from_hex_str(value: &str) -> Result<Self> {
        let key = SecretKey::from_slice(value.try_into_bytes()?.as_ref())?;

        Ok(vec![key].into())
    }

    /// Create a signer with bip32 flags.
    ///
    /// # Accounts drive path
    ///
    /// All accounts created by this function are drive by path with prefix "m/44'/60'/0'/0/".
    /// the first account is drived by path "m/44'/60'/0'/0/0", the second account is drived by
    /// path "m/44'/60'/0'/0/1" and so on.
    pub fn from_bip32_wallet<M, P>(mnemonic: M, password: P, accounts: usize) -> Result<Self>
    where
        M: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        let drive_key = DriveKey::new(mnemonic, password);

        let mut keys = vec![];

        for id in 0..accounts {
            keys.push(
                drive_key
                    .drive(format!("m/44'/60'/0'/0/{}", id))?
                    .private_key,
            );
        }

        Ok(keys.into())
    }

    /// Create a signer for hardhat network.
    #[cfg(feature = "hardhat")]
    #[cfg_attr(docsrs, doc(cfg(feature = "hardhat")))]
    pub fn hardhat() -> Result<Self> {
        Self::from_bip32_wallet(
            "test test test test test test test test test test test junk",
            "",
            10,
        )
    }
}

#[cfg(test)]
mod tests {

    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::eip::eip2718::{Eip2930TransactionRequest, LegacyTransactionRequest};

    use super::*;

    #[futures_test::test]
    async fn test_sign_tx() {
        let _ = pretty_env_logger::try_init();

        let signer = WalletSigner::from_hex_str(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .expect("Create hardhat account 0 wallet");

        let tx: LegacyTransactionRequest = serde_json::from_value(json!({
            "nonce": "0x1",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "value":"0x1",
            "data":"0x",
            "gas":"0x60000",
            "gasPrice": "0x60000111"

        }))
        .expect("Create tx");

        let signed_tx = signer
            .sign_transaction(tx.clone(), None)
            .await
            .expect("Sign tx");

        assert_eq!(
            "0xf864018460000111830600009470997970c51812dc3a010c7d01b50e0d17dc79c8018026a06c7e1e13070e6f10e51d7d20e986c59fd080fc6afc5508f44e8b0a84a58b7d1aa013c20fa2b6d77ae6814a41b674946387dde6401c73eb0cab2246a2981c48e344",
            signed_tx.to_string(),
        );

        let tx: Eip2930TransactionRequest = serde_json::from_value(json!({
            "type": "0x01",
            "nonce": "0x1",
            "to": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
            "value":"0x1",
            "data":null,
            "gas":"0x60000",
            "gasPrice": "0x60000111",
            "chainId": "0x1",
            "accessList":[]

        }))
        .expect("Create tx");

        let signed_tx = signer
            .sign_transaction(tx.clone(), None)
            .await
            .expect("Sign tx");

        assert_eq!(
            "0x01f86601018460000111830600009470997970c51812dc3a010c7d01b50e0d17dc79c80180c080a0fd6402ef803609fce890c09304abd681fe29c25616e960c1f44db1a026f5d03ba00867e433e4ddd6e8b0c8f283c08a7f81d2cc41cc29aaa8631d7b979a8ec9e8ec",
            signed_tx.to_string()
        );
    }

    #[futures_test::test]
    async fn test_sign_eip712() {
        _ = pretty_env_logger::try_init();

        let signer = WalletSigner::from_hex_str(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .expect("Create hardhat account 0 wallet");

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Person {
            pub name: String,
            pub wallet: Address,
        }

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct Mail {
            pub from: Person,
            pub to: Person,
            pub contents: String,
        }

        let mail: TypedData<Mail> = serde_json::from_str(include_str!("./eip712.json")).unwrap();

        let signature = signer
            .sign_typed_data(mail, None)
            .await
            .expect("Sign typed_data mail");

        assert_eq!(signature.to_string(),"0x006ea8bb309a3401225701f3565e32519f94a0ea91a5910ce9229fe488e773584c0390416a2190d9560219dab757ecca2029e63fa9d1c2aebf676cc25b9f03126a");
    }
}
