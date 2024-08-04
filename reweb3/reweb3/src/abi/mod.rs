//! The encoding/decoding support for [`contract abi`](https://docs.soliditylang.org/en/develop/abi-spec.html).
//!
//! This module provide to function to encode/decode rust value:
//! - [`from_abi`]: serialize rust object into solidity(contract) abi format.
//! - [`to_abi`]: deserialize rust object from solidity(contract) abi format.
//!
//! By design, these two functions cannot distinguish between the underlying reference types of json values,
//! and therefore cannot pass [`serde_json::Value`] to them. Of course you could look at [`eip712`](crate::eip::eip712) and provide
//! additional type definition parameters to enable dynamic transforming, but that's a piece of the design goal that
//! isn't a concern for `reweb3`.
//!
//! # Static
//!
//! reweb3 only supports "contract abi" static encoder/decoder via [`serde`] framework:
//!
//! ```no_run
//! # fn main() {
//! # use ::serde::{Deserialize, Serialize};
//! # use serde_json::json;
//! # use reweb3::primitives::Address;
//! # use reweb3::abi::*;
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
//! let mail: Mail = serde_json::from_value(json).unwrap();
//!
//! let mail_decoded: Mail = from_abi(to_abi(&mail).unwrap()).unwrap();
//!
//! assert_eq!(mail_decoded, mail);
//!
//! let seq: Vec<u32> = from_abi(to_abi(&vec![1, 1, 1]).unwrap()).unwrap();
//!
//! assert_eq!(seq, vec![1, 1, 1]);
//! # }
//! ```

mod de;

mod ser;

pub use de::*;
pub use ser::*;

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use crate::primitives::Address;

    use super::*;

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

    #[test]
    fn encode_decode() {
        let json = json!({
          "from": {
            "name": "Cow",
            "wallet": "0xCD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826"
          },
          "to": {
            "name": "Bob",
            "wallet": "0xbBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB"
          },
          "contents": "Hello, Bob!"
        });

        let mail: Mail = serde_json::from_value(json).unwrap();

        let mail_decoded: Mail = from_abi(to_abi(&mail).unwrap()).unwrap();

        assert_eq!(mail_decoded, mail);

        let seq: Vec<u32> = from_abi(to_abi(&vec![1, 1, 1]).unwrap()).unwrap();

        assert_eq!(seq, vec![1, 1, 1]);
    }
}
