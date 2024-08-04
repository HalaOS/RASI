use serde::{Deserialize, Serialize};

use crate::primitives::{Address, H256};

/// See [`JSON-RPC Specification`](https://ethereum.github.io/execution-apis/api-documentation/) for details.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AccessList(pub Vec<Access>);

impl Serialize for AccessList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for AccessList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let accesses = Vec::<Access>::deserialize(deserializer)?;

        Ok(Self(accesses))
    }
}

/// See [`JSON-RPC Specification`](https://ethereum.github.io/execution-apis/api-documentation/) for details.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Access {
    pub address: Address,

    pub storage_keys: Vec<H256>,
}
