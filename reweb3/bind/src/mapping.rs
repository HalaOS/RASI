//! The mapping utilities between solidity type and target strong type language.
//!

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::typedef::{Array, ArrayM, FixedMN, IntegerM, Type};

/// The context to map solidity type to target strong type language.
/// you can read mapping data from any [`serde`] compatibable data formats.
#[derive(Debug, Serialize, Deserialize)]
pub struct BinderTypeMapping {
    #[serde(flatten)]
    mapping: HashMap<String, String>,

    #[serde(skip)]
    dynamic_mapping: HashMap<String, String>,
}

impl BinderTypeMapping {
    /// Mapping abi type to target language type.
    pub fn abi_type_mapping(&mut self, abi_type: &Type) -> Option<&str> {
        match abi_type {
            // for tuple type always returns None.
            Type::Simple(el) if el.is_tuple() => return None,
            Type::Simple(el) => return self.get(el.to_string().as_str()),
            Type::BytesM(el) => self.bytes_m_mapping(el.m),
            Type::IntegerM(el) => self.integer_m_mapping(el),
            Type::FixedMN(el) => self.fixed_m_n_mapping(el),
            Type::ArrayM(el) => return self.array_m_mapping(el),
            Type::Array(el) => self.array_mapping(el),
        }
    }

    /// Mapping binder-specific runtime type into target language type.
    ///
    /// `runtime Type` is the runtime type of the target web3 framework that must be known to generate code.
    pub fn rt_type_mapping(&self, rt_type: &str) -> Option<&str> {
        self.get(rt_type)
    }

    fn get(&self, type_name: &str) -> Option<&str> {
        self.mapping.get(type_name).map(String::as_str)
    }

    fn dynamic_mapping<T: ToString>(&self, t: T) -> Option<&str> {
        self.dynamic_mapping
            .get(t.to_string().as_str())
            .map(String::as_str)
    }

    fn integer_m_mapping(&mut self, el: &IntegerM) -> Option<&str> {
        let tag = el.to_string();

        // Returns exists static mapping type.
        if self.mapping.contains_key(&tag) {
            return self.mapping.get(&tag).map(String::as_str);
        }

        // Returns exists runtime type
        if self.dynamic_mapping.contains_key(&tag) {
            return self.dynamic_mapping.get(&tag).map(|c| c.as_str());
        }

        let runtime_type = if el.signed {
            self.get("int_m")?
        } else {
            self.get("uint_m")?
        };

        let declare_type = runtime_type.replace("$m", &el.m.to_string());

        self.dynamic_mapping.insert(tag, declare_type);

        self.dynamic_mapping(el)
    }

    fn array_m_mapping(&mut self, el: &ArrayM) -> Option<&str> {
        let tag = el.to_string();

        // Returns exists static mapping type.
        if self.mapping.contains_key(&tag) {
            return self.mapping.get(&tag).map(String::as_str);
        }

        // Returns exists runtime type
        if self.dynamic_mapping.contains_key(&tag) {
            return self.dynamic_mapping.get(&tag).map(|c| c.as_str());
        }

        let el_type = self.abi_type_mapping(&el.element)?.to_owned();

        let runtime_type = self.get("array_m")?;

        let m = el.m;

        let declare_type = runtime_type
            .replace("$el", &el_type)
            .replace("$m", &m.to_string());

        self.dynamic_mapping.insert(tag, declare_type);

        self.dynamic_mapping(el)
    }

    fn fixed_m_n_mapping(&mut self, el: &FixedMN) -> Option<&str> {
        let tag = el.to_string();

        // Returns exists static mapping type.
        if self.mapping.contains_key(&tag) {
            return self.mapping.get(&tag).map(String::as_str);
        }

        // Returns exists runtime type
        if self.dynamic_mapping.contains_key(&tag) {
            return self.dynamic_mapping.get(&tag).map(|c| c.as_str());
        }

        let runtime_type = self.get("fixed_m_n")?;

        let declare_type = runtime_type
            .replace("$m", &el.m.to_string())
            .replace("$n", &el.n.to_string());

        self.dynamic_mapping.insert(tag, declare_type);

        self.dynamic_mapping(el)
    }

    fn array_mapping(&mut self, el: &Array) -> Option<&str> {
        let tag = el.to_string();

        // Returns exists runtime type
        if self.dynamic_mapping.contains_key(&tag) {
            return self.dynamic_mapping.get(&tag).map(|c| c.as_str());
        }

        let el_type = self.abi_type_mapping(&el.element)?.to_owned();

        let runtime_type = self.get("array")?;

        let declare_type = runtime_type.replace("$el", &el_type);

        self.dynamic_mapping.insert(tag, declare_type);

        self.dynamic_mapping(el)
    }

    fn bytes_m_mapping(&mut self, m: usize) -> Option<&str> {
        let tag = format!("array{}", m);

        // Returns exists static mapping type.
        if self.mapping.contains_key(&tag) {
            return self.mapping.get(&tag).map(String::as_str);
        }

        // Returns exists runtime type
        if self.dynamic_mapping.contains_key(&tag) {
            return self.dynamic_mapping.get(&tag).map(String::as_str);
        }

        let runtime_type = self.get("bytes_m")?;

        let declare_type = runtime_type.replace("$m", &m.to_string());

        self.dynamic_mapping.insert(tag.clone(), declare_type);

        self.dynamic_mapping(tag)
    }
}
