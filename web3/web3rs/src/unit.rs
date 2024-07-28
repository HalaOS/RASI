use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use crate::errors::{Error, Result};

mod u256 {
    use uint::construct_uint;
    construct_uint! {
        /// Unsigned int with 256 bits length.
        pub struct U256(4);
    }
}

pub use u256::U256;

/// Unit for ethereum account balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unit {
    Wei,
    Kwei,
    Mwei,
    Gwei,
    Szabo,
    Finney,
    Ether,
}

impl Unit {
    pub fn decimals(&self) -> usize {
        match self {
            Unit::Wei => 0,
            Unit::Kwei => 3,
            Unit::Mwei => 6,
            Unit::Gwei => 9,
            Unit::Szabo => 12,
            Unit::Finney => 15,
            Unit::Ether => 18,
        }
    }
}

impl Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unit::Wei => write!(f, "wei"),
            Unit::Kwei => write!(f, "kwei"),
            Unit::Mwei => write!(f, "mwei"),
            Unit::Gwei => write!(f, "gwei"),
            Unit::Szabo => write!(f, "szabo"),
            Unit::Finney => write!(f, "finney"),
            Unit::Ether => write!(f, "ether"),
        }
    }
}

impl FromStr for Unit {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_ref() {
            "wei" => Ok(Self::Wei),
            "kwei" => Ok(Self::Kwei),
            "mwei" => Ok(Self::Mwei),
            "gwei" => Ok(Self::Gwei),
            "szabo" => Ok(Self::Szabo),
            "finney" => Ok(Self::Finney),
            "ether" => Ok(Self::Ether),
            u => Err(Error::ParseUnit(u.to_owned())),
        }
    }
}

impl TryFrom<&str> for Unit {
    type Error = Error;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        value.parse()
    }
}

/// A Unit can be specified as a fixed decimals number,
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal {
    bignumber: U256,
    unit: Unit,
}

impl FromStr for Decimal {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let splits = s.trim().split_whitespace().collect::<Vec<_>>();

        if splits.len() != 2 {
            return Err(Error::ParseDecimal(s.to_owned()));
        }

        let unit = Unit::from_str(splits[1])?;
        let bignumber = U256::from_dec_str(splits[0])? * U256::from(10).pow(unit.decimals().into());

        Ok(Self { bignumber, unit })
    }
}

impl From<Decimal> for U256 {
    fn from(value: Decimal) -> Self {
        value.bignumber
    }
}

impl From<&Decimal> for U256 {
    fn from(value: &Decimal) -> Self {
        value.bignumber
    }
}

impl Decimal {
    /// Parse string as `Decimal`
    pub fn parse<S: AsRef<str>, U: TryInto<Unit>>(value: S, unit: U) -> Result<Self>
    where
        U::Error: ToString,
    {
        let unit: Unit = unit
            .try_into()
            .map_err(|err| Error::ParseUnit(err.to_string()))?;

        let bignumber =
            U256::from_dec_str(value.as_ref())? * U256::from(10).pow(unit.decimals().into());

        Ok(Self { bignumber, unit })
    }
}

#[cfg(test)]
mod tests {

    use crate::unit::{Decimal, Unit};

    use super::U256;

    #[test]
    fn test_parse_str() {
        assert_eq!(U256::from_dec_str("100").unwrap(), U256::from(100));

        assert_eq!(
            Decimal::parse("100", Unit::Ether).unwrap(),
            Decimal::parse("100", "ether").unwrap(),
        );

        assert_eq!(
            U256::from(Decimal::parse("100", Unit::Kwei).unwrap()),
            U256::from(100000)
        );
    }
}
