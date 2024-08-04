//! Hashing and signing of typed structured data as opposed to just bytestrings.
//!
//! # static
//!
//! This module extends [`serde`] as the serialisation/deserialisation framework for eip712.
//!
//! ```no_run
//! # fn main() {
//! use reweb3::eip::eip712::*;
//! use ::serde::{Deserialize, Serialize};
//! use ::serde_json::json;
//! use reweb3::primitives::Address;
//!
//! let domain = json!({
//!     "name": "Ether Mail",
//!     "version": "1",
//!     "chainId": 1,
//!     "verifyingContract": "0xCcCCccccCCCCcCCCCCCcCcCccCcCCCcCcccccccC",
//! });
//!
//! let domain: EIP712Domain = serde_json::from_value(domain).unwrap();
//!
//! let type_definitions = serde::eip712_type_definitions(&domain).unwrap();
//!
//! assert_eq!(
//!     serde::eip712_hash_struct("EIP712Domain", &type_definitions, &domain)
//!         .unwrap()
//!         .to_string(),
//!     "0xf2cee375fa42b42143804025fc449deafd50cc031ca257e0b194a650a912090f"
//! );
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Person {
//!     pub name: String,
//!     pub wallet: Address,
//! }
//!
//! #[derive(Debug, Serialize, Deserialize, PartialEq)]
//! struct Mail {
//!     pub from: Person,
//!     pub to: Person,
//!     pub contents: String,
//! }
//!
//! let json = json!({
//!   "from": {
//!     "name": "Cow",
//!     "wallet": "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826"
//!   },
//!   "to": {
//!     "name": "Bob",
//!     "wallet": "0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB"
//!   },
//!   "contents": "Hello, Bob!"
//! });
//!
//! let mail: Mail = serde_json::from_value(json).expect("parse domain");
//!
//!  assert_eq!(
//!      "Mail(Person from,Person to,string contents)Person(string name,address wallet)",
//!      serde::eip712_encode_type(&mail).expect("generate e712 types")
//!  );
//!
//!  let expect_request: TypedData<Mail> =
//!      serde_json::from_str(include_str!("./eip712.json")).unwrap();
//!
//!  assert_eq!(eip712_into_request(domain, mail).unwrap(), expect_request);
//!
//!  assert_eq!(
//!      expect_request.sign_hash().unwrap(),
//!      "0xbe609aee343fb3c4b28e1df9e632fca64fcfaede20f02e86244efddf30957bd2"
//!          .parse()
//!          .unwrap()
//!  );
//! # }
//! ```
//! # Dynamic
//!
//! With [`serde_json`] this module provides ***limited dynamic reflection*** capabilities:
//!
//! ```no_run
//! # fn main() {
//! # use reweb3::eip::eip712::*;
//! // Use `serde_json::Value` to create a TypedData
//! let expect_request: TypedData<serde_json::Value> =
//!     serde_json::from_str(include_str!("./eip712.json")).unwrap();
//!
//! assert_eq!(
//!     expect_request.sign_hash().unwrap(),
//!     "0xbe609aee343fb3c4b28e1df9e632fca64fcfaede20f02e86244efddf30957bd2"
//!         .parse()
//!         .unwrap()
//! );
//! # }
//! ```

pub mod serde;

mod eip712;
pub use eip712::*;
