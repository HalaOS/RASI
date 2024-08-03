//! Ethereum constract abi json format encode/decode support
//!
//! Visit [`official document`](https://docs.soliditylang.org/en/v0.8.17/abi-spec.html#json) for details

use std::{fmt::Display, str::FromStr};

use regex::Regex;
use serde::{Deserialize, Serialize};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AbiError {
    #[error("Invalid fixed type declare {0}, {1}")]
    FixedMN(String, String),

    #[error("Invalid integer type declare {0}, {1}")]
    IntegerM(String, String),

    #[error("Invalid fixed length binary type declare {0}, {1}")]
    BytesM(String, String),

    #[error("Invalid tuple type declare {0}, {1}")]
    Tuple(String, String),

    #[error("Invalid fixed-length Array type declare {0}, {1}")]
    ArrayM(String, String),

    #[error("Invalid Array type declare {0}, {1}")]
    Array(String, String),

    #[error("Invalid Type declare {0}")]
    UnknownType(String),
}

/// Hardhat generate artifact
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardhatArtifact {
    pub contract_name: String,
    pub source_name: String,
    pub abi: Vec<AbiField>,
    pub bytecode: String,
    pub deployed_bytecode: String,
}

/// Contract interface type enum
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum AbiField {
    Function(Function),
    Constructor(Constructor),
    Receive(Receive),
    Fallback(Fallback),
    Event(Event),
    Error(Error),
}

/// A structure type to represent `function` abi
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
    /// the function name
    pub name: String,
    /// An array of function's input params
    #[serde(default = "default_parameters")]
    pub inputs: Vec<Parameter>,
    /// An array of function's output params
    #[serde(default = "default_parameters")]
    pub outputs: Vec<Parameter>,
    /// a string with one of the following values: pure (specified to not read blockchain state),
    /// view (specified to not modify the blockchain state),
    /// nonpayable (function does not accept Ether - the default) and payable (function accepts Ether)
    pub state_mutability: StateMutability,
}

impl Function {
    /// Only include inputs,e.g: withdraw(address)
    pub fn signature(&self) -> String {
        let tuple = Self::to_signature(&self.inputs);

        format!("{}{}", self.name, tuple)
    }

    fn to_signature(params: &[Parameter]) -> String {
        let mut pairs = vec![];

        for param in params.iter() {
            if let Some(components) = &param.components {
                let element = Self::to_signature(components);
                match &param.r#type {
                    Type::Array(_) => {
                        pairs.push(format!("{}[]", element));
                    }
                    Type::ArrayM(array_m) => {
                        pairs.push(format!("{}[{}]", element, array_m.m));
                    }
                    _ => {
                        pairs.push(format!("{}", element));
                    }
                }
            } else {
                pairs.push(format!("{}", param.r#type));
            }
        }

        format!("({})", pairs.join(","))
    }
}

fn default_parameters() -> Vec<Parameter> {
    vec![]
}

/// A structure type to represent `constructor` abi
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Constructor {
    /// An array of function's input params
    pub inputs: Vec<Parameter>,
    /// a string with one of the following values: pure (specified to not read blockchain state),
    /// view (specified to not modify the blockchain state),
    /// nonpayable (function does not accept Ether - the default) and payable (function accepts Ether)
    pub state_mutability: StateMutability,
}

impl Constructor {
    /// Only include inputs,e.g: withdraw(address)
    pub fn signature(&self) -> String {
        let tuple = Self::to_signature(&self.inputs);

        format!("Constructor{}", tuple)
    }

    fn to_signature(params: &[Parameter]) -> String {
        let mut pairs = vec![];

        for param in params.iter() {
            if let Some(components) = &param.components {
                let element = Self::to_signature(components);
                match &param.r#type {
                    Type::Array(_) => {
                        pairs.push(format!("{}[]", element));
                    }
                    Type::ArrayM(array_m) => {
                        pairs.push(format!("{}[{}]", element, array_m.m));
                    }
                    _ => {
                        pairs.push(format!("{}", element));
                    }
                }
            } else {
                pairs.push(format!("{}", param.r#type));
            }
        }

        format!("({})", pairs.join(","))
    }
}

/// A structure type to represent `receive function` abi
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receive {
    /// a string with one of the following values: pure (specified to not read blockchain state),
    /// view (specified to not modify the blockchain state),
    /// nonpayable (function does not accept Ether - the default) and payable (function accepts Ether)
    pub state_mutability: StateMutability,
}

/// A structure type to represent `fallback function` abi
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fallback {
    /// a string with one of the following values: pure (specified to not read blockchain state),
    /// view (specified to not modify the blockchain state),
    /// nonpayable (function does not accept Ether - the default) and payable (function accepts Ether)
    pub state_mutability: StateMutability,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StateMutability {
    Pure,
    View,
    Nonpayable,
    Payable,
}

/// A structure type to represent `event` abi
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// the function name
    pub name: String,
    /// An array of function's input params
    pub inputs: Vec<Parameter>,
    /// `true` if the event was declared as anonymous
    pub anonymous: bool,
}

/// A structure type to represent `event` abi
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    /// the function name
    pub name: String,
    /// An array of function's input params
    pub inputs: Vec<Parameter>,
}
/// Handle Function/Event/Error 's input or output parameter type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parameter {
    /// The name of the parameter
    pub name: String,
    /// The canonical type of the parameter
    pub r#type: Type,
    /// used for tuple types, only if the type field start with prefix `tuple`. e.g, `tupe[]`,`tuple`
    pub components: Option<Vec<Parameter>>,
    /// This field is only meaningful for `Event` or `Error`
    #[serde(default = "default_indexed")]
    pub indexed: bool,
    /// Hardhat extension field
    pub internal_type: Option<String>,
}

fn default_indexed() -> bool {
    false
}

/// Contract abi simple types enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SimpleType {
    Address,
    Uint,
    Int,
    Bool,
    Fixed,
    Ufixed,
    /// an address (20 bytes) followed by a function selector (4 bytes). Encoded identical to bytes24.
    Function,
    Bytes,
    String,
    Tuple,
}

impl ToString for SimpleType {
    fn to_string(&self) -> String {
        // to canonical type name
        match self {
            Self::Ufixed => "fixed128x18".into(),
            Self::Fixed => "ufixed128x18".into(),
            Self::Int => "int256".into(),
            Self::Uint => "uint256".into(),
            _ => {
                let data = serde_json::to_string(self).unwrap();

                data[1..data.len() - 1].to_string()
            }
        }
    }
}

impl SimpleType {
    pub fn is_tuple(&self) -> bool {
        match self {
            Self::Tuple => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// fixed-point decimal number of M bits, 8 <= M <= 256, M % 8 == 0, and 0 < N <= 80, which denotes the value v as v / (10 ** N).
pub struct FixedMN {
    pub m: usize,
    pub n: usize,
    pub signed: bool,
}

impl Display for FixedMN {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_string(self).expect("Serialize type to json");
        write!(f, "{}", &value[1..value.len() - 1])
    }
}

impl Serialize for FixedMN {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.signed {
            serializer.serialize_str(&format!("fixed{}x{}", self.m, self.n))
        } else {
            serializer.serialize_str(&format!("ufixed{}x{}", self.m, self.n))
        }
    }
}

fn fixed_regex() -> Regex {
    Regex::new(r"^(u){0,1}fixed(\d{1,3})x(\d{1,3})$").unwrap()
}

impl<'de> Deserialize<'de> for FixedMN {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;

        if let Some(captures) = fixed_regex().captures(&data) {
            let signed = captures.get(1).map(|_| false).unwrap_or(true);

            let m: usize = (&captures[2]).parse().map_err(serde::de::Error::custom)?;
            let n: usize = (&captures[3]).parse().map_err(serde::de::Error::custom)?;

            if m < 8 || m > 256 || m % 8 != 0 {
                return Err(AbiError::FixedMN(
                    data,
                    "M bits must meet the condition 0 < M <= 256, M % 8 == 0".to_string(),
                ))
                .map_err(serde::de::Error::custom);
            }

            if n > 80 {
                return Err(AbiError::FixedMN(
                    data,
                    "decimal numbers N must meet the condition 0 < N <= 80".to_string(),
                ))
                .map_err(serde::de::Error::custom);
            }

            Ok(Self { signed, m, n })
        } else {
            return Err(AbiError::FixedMN(
                data,
                "{u}fixed<M>x<N>: fixed-point decimal number of M bits, 8 <= M <= 256, M % 8 == 0, and 0 < N <= 80"
                    .to_string(),
            ))
            .map_err(serde::de::Error::custom);
        }
    }
}

/// integer type of M bits, 0 < M <= 256, M % 8 == 0. e.g. uint32, uint8
#[derive(Debug, Clone, PartialEq)]
pub struct IntegerM {
    pub signed: bool,
    pub m: usize,
}

impl Display for IntegerM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_string(self).expect("Serialize type to json");
        write!(f, "{}", &value[1..value.len() - 1])
    }
}

impl Serialize for IntegerM {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.signed {
            serializer.serialize_str(&format!("int{}", self.m,))
        } else {
            serializer.serialize_str(&format!("uint{}", self.m,))
        }
    }
}

fn integer_regex() -> Regex {
    Regex::new(r"^(u){0,1}int(\d{1,3})$").unwrap()
}

impl<'de> Deserialize<'de> for IntegerM {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;

        if let Some(captures) = integer_regex().captures(&data) {
            let signed = captures.get(1).map(|_| false).unwrap_or(true);

            let m: usize = (&captures[2]).parse().map_err(serde::de::Error::custom)?;

            if m < 8 || m > 256 || m % 8 != 0 {
                return Err(AbiError::IntegerM(
                    data,
                    "M bits must meet the condition 0 < M <= 256, M % 8 == 0".to_string(),
                ))
                .map_err(serde::de::Error::custom);
            }

            Ok(Self { signed, m })
        } else {
            return Err(AbiError::FixedMN(
                data,
                "{u}int<M>: unsigned integer type of M bits, 0 < M <= 256, M % 8 == 0".to_string(),
            ))
            .map_err(serde::de::Error::custom);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// binary type of M bytes, 0 < M <= 32
pub struct BytesM {
    pub m: usize,
}

impl Display for BytesM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_string(self).expect("Serialize type to json");
        write!(f, "{}", &value[1..value.len() - 1])
    }
}

impl Serialize for BytesM {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("bytes{}", self.m,))
    }
}

impl<'de> Deserialize<'de> for BytesM {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;

        if data.starts_with("bytes") {
            let m: usize = (&data[5..]).parse().map_err(serde::de::Error::custom)?;

            if m > 32 {
                return Err(AbiError::BytesM(data, "0 < M <= 32".to_string()))
                    .map_err(serde::de::Error::custom);
            }

            Ok(Self { m })
        } else {
            return Err(AbiError::BytesM(
                data,
                "bytes<M>: binary type of M bytes, 0 < M <= 32".to_string(),
            ))
            .map_err(serde::de::Error::custom);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Simple(SimpleType),

    BytesM(BytesM),

    IntegerM(IntegerM),

    FixedMN(FixedMN),

    ArrayM(Box<ArrayM>),
    Array(Box<Array>),
}

impl From<Type> for String {
    fn from(value: Type) -> Self {
        let str = serde_json::to_string(&value).unwrap();

        str[1..str.len() - 1].to_owned()
    }
}

impl FromStr for Type {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(&format!("\"{}\"", s))
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_string(self).expect("Serialize type to json");
        write!(f, "{}", &value[1..value.len() - 1])
    }
}

impl Serialize for Type {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Simple(simple) => simple.serialize(serializer),
            Self::BytesM(byte_m) => byte_m.serialize(serializer),
            Self::IntegerM(integer_m) => integer_m.serialize(serializer),
            Self::FixedMN(fixed_m) => fixed_m.serialize(serializer),
            Self::ArrayM(array_m) => array_m.serialize(serializer),
            Self::Array(array) => array.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Type {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = String::deserialize(deserializer)?;

        let data = format!("\"{}\"", data);

        if let Ok(array_m) = serde_json::from_str::<ArrayM>(&data) {
            return Ok(Self::ArrayM(Box::new(array_m)));
        }

        if let Ok(array) = serde_json::from_str::<Array>(&data) {
            return Ok(Self::Array(Box::new(array)));
        }

        if let Ok(fixed_m_n) = serde_json::from_str::<FixedMN>(&data) {
            return Ok(Self::FixedMN(fixed_m_n));
        }

        if let Ok(integer_m) = serde_json::from_str::<IntegerM>(&data) {
            return Ok(Self::IntegerM(integer_m));
        }

        if let Ok(bytes_m) = serde_json::from_str::<BytesM>(&data) {
            return Ok(Self::BytesM(bytes_m));
        }

        if let Ok(simple_type) = serde_json::from_str::<SimpleType>(&data) {
            return Ok(Self::Simple(simple_type));
        }

        return Err(AbiError::UnknownType(data)).map_err(serde::de::Error::custom);
    }
}

#[derive(Debug, Clone, PartialEq)]
/// a fixed-length array of M elements, M >= 0, of the given type.
pub struct ArrayM {
    pub element: Type,
    /// fixed-length array of `m` elements, M >= 0,
    pub m: usize,
}

impl Display for ArrayM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_string(self).expect("Serialize type to json");
        write!(f, "{}", &value[1..value.len() - 1])
    }
}

impl Serialize for ArrayM {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let element = serde_json::to_string(&self.element).map_err(serde::ser::Error::custom)?;

        serializer.serialize_str(&format!("{}[{}]", &element[1..element.len() - 1], self.m))
    }
}

fn array_m_regex() -> Regex {
    Regex::new(r"\[(\d{1,3})\]$").unwrap()
}

impl<'de> Deserialize<'de> for ArrayM {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let array_m = String::deserialize(deserializer)?;

        let end_with_regex = array_m_regex();

        if let Some(caps) = end_with_regex.captures(&array_m) {
            let m: usize = (&caps[1]).parse().map_err(serde::de::Error::custom)?;

            let data = format!("\"{}\"", &array_m[..array_m.len() - caps.len() - 2]);

            let element: Type = serde_json::from_str(&data).map_err(serde::de::Error::custom)?;

            return Ok(Self { element, m });
        } else {
            return Err(AbiError::ArrayM(
                array_m,
                "<type>[M]: a fixed-length array of M elements, M >= 0, of the given type"
                    .to_string(),
            ))
            .map_err(serde::de::Error::custom);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// a variable-length array of elements of the given type
pub struct Array {
    pub element: Type,
}

impl Display for Array {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_string(self).expect("Serialize type to json");
        write!(f, "{}", &value[1..value.len() - 1])
    }
}

impl Serialize for Array {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let element = serde_json::to_string(&self.element).map_err(serde::ser::Error::custom)?;

        serializer.serialize_str(&format!("{}[]", &element[1..element.len() - 1]))
    }
}

impl<'de> Deserialize<'de> for Array {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let array_m = String::deserialize(deserializer)?;

        if array_m.ends_with("[]") {
            let data = format!("\"{}\"", &array_m[..array_m.len() - 2]);
            let element: Type = serde_json::from_str(&data).map_err(serde::de::Error::custom)?;

            return Ok(Self { element });
        } else {
            return Err(AbiError::Array(
                array_m,
                "<type>[]: a variable-length array of elements of the given type.".to_string(),
            ))
            .map_err(serde::de::Error::custom);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_fixed_regex() {
        _ = pretty_env_logger::try_init();
        let re = fixed_regex();

        assert!(re.is_match("ufixed100x18"));

        assert!(re.is_match("fixed100x18"));

        assert!(!re.is_match("fixed1000x18"));

        assert!(!re.is_match("ufixed1000x18"));
        assert!(!re.is_match("uufixed1000x18"));

        assert!(!re.is_match("fixed-100x18"));

        if let Some(captures) = fixed_regex().captures("fixed128x18") {
            assert_eq!(captures.get(1), None);
            assert_eq!(captures.get(2).map(|c| c.as_str()), Some("128"));
            assert_eq!(captures.get(3).map(|c| c.as_str()), Some("18"));
        }
    }

    #[test]
    fn test_fixed_json() {
        let fixed: FixedMN = serde_json::from_str(r#""fixed128x18""#).expect("Parse fixed");

        assert_eq!(fixed.signed, true);
        assert_eq!(fixed.m, 128);
        assert_eq!(fixed.n, 18);

        let fixed: FixedMN = serde_json::from_str(r#""ufixed128x18""#).expect("Parse fixed");

        assert_eq!(fixed.signed, false);
        assert_eq!(fixed.m, 128);
        assert_eq!(fixed.n, 18);

        serde_json::from_str::<FixedMN>(r#""ufixed100x18""#).expect_err("M % 8 == 0");

        serde_json::from_str::<FixedMN>(r#""ufixed128x180""#).expect_err("N <= 80");
    }

    #[test]
    fn test_int_json() {
        let fixed: IntegerM = serde_json::from_str(r#""int128""#).expect("Parse integer");

        assert_eq!(fixed.signed, true);
        assert_eq!(fixed.m, 128);

        let fixed: IntegerM = serde_json::from_str(r#""uint128""#).expect("Parse integer");

        assert_eq!(fixed.signed, false);
        assert_eq!(fixed.m, 128);

        serde_json::from_str::<IntegerM>(r#""uint100""#).expect_err("M % 8 == 0");
    }

    #[test]
    fn test_end_with() {
        let end_with_regex = array_m_regex();

        let caps = end_with_regex.captures("Hello[1][123]").unwrap();

        assert_eq!(&caps[1], "123");
    }

    #[test]
    fn test_type_serde() {
        _ = pretty_env_logger::try_init();
        fn check(expect: &str) {
            let t: Type = expect.parse().expect("Parse type string");

            let data: String = t.into();

            assert_eq!(data, expect);
        }

        let test_vector = vec![
            "uint256",
            "int256",
            "address",
            "int8",
            "uint",
            "int",
            "bool",
            "fixed128x16",
            "ufixed128x16",
            "fixed",
            "ufixed",
            "bytes",
            "bytes24",
            "tuple",
            "function",
            "string",
            "tuple[]",
            "tuple[][32]",
            "bool[20]",
            "uint256[20]",
        ];

        for v in test_vector {
            check(v);
        }
    }

    #[test]
    fn test_hardhat_artifact() {
        let _: HardhatArtifact =
            serde_json::from_str(include_str!("abi.json")).expect("Parse hardhat artifact");
    }

    #[test]
    fn test_field() {
        let data = r#"
             {
      "inputs": [
        {
          "internalType": "address",
          "name": "WETH_",
          "type": "address"
        }
      ],
      "stateMutability": "nonpayable",
      "type": "constructor"
    }
        "#;

        _ = serde_json::from_str::<AbiField>(data).expect("Parse abi field");
    }
}
