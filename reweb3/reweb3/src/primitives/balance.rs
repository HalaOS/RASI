use std::num::ParseIntError;

use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

use super::U256;

#[derive(Debug, thiserror::Error)]
pub enum ParseBalanceError {
    #[error("Invalid ethereum unit type: {0}")]
    ParseUnit(String),

    #[error("Decimal base part is empty.")]
    BasePartIsEmpty,

    #[error("Exponent overflow when parsing {0}")]
    ExponentOverflow(String),

    #[error("Decimal unit part is empty.")]
    UnitIsEmpty,

    #[error("Decimal overflow when parsing {0}")]
    OverFlow(String),

    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),

    #[error("{0}")]
    Other(String),
}

/// Result type for balance mod.
pub type Result<T> = std::result::Result<T, ParseBalanceError>;

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
    pub fn decimals(&self) -> i64 {
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
    type Err = ParseBalanceError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_ref() {
            "wei" => Ok(Self::Wei),
            "kwei" => Ok(Self::Kwei),
            "mwei" => Ok(Self::Mwei),
            "gwei" => Ok(Self::Gwei),
            "szabo" => Ok(Self::Szabo),
            "finney" => Ok(Self::Finney),
            "ether" => Ok(Self::Ether),
            u => Err(ParseBalanceError::ParseUnit(u.to_owned())),
        }
    }
}

impl TryFrom<&str> for Unit {
    type Error = ParseBalanceError;
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        value.parse()
    }
}

fn parse_dec_str(value: &str, unit: Unit) -> Result<Decimals> {
    let exp_separator: &[_] = &['e', 'E'];

    // split slice into base and exponent parts
    let (base_part, exponent_value) = match value.find(exp_separator) {
        // exponent defaults to 0 if (e|E) not found
        None => (value, 0),

        // split and parse exponent field
        Some(loc) => {
            // slice up to `loc` and 1 after to skip the 'e' char
            let (base, e_exp) = value.split_at(loc);
            (base, i128::from_str(&e_exp[1..])?)
        }
    };

    if base_part.is_empty() {
        return Err(ParseBalanceError::BasePartIsEmpty);
    }

    let mut digit_buffer = String::new();

    let last_digit_loc = base_part.len() - 1;

    // split decimal into a digit string and decimal-point offset
    let (digits, decimal_offset) = match base_part.find('.') {
        // No dot! pass directly to BigInt
        None => (base_part, 0),
        // dot at last digit, pass all preceding digits to BigInt
        Some(loc) if loc == last_digit_loc => (&base_part[..last_digit_loc], 0),
        // decimal point found - necessary copy into new string buffer
        Some(loc) => {
            // split into leading and trailing digits
            let (lead, trail) = (&base_part[..loc], &base_part[loc + 1..]);

            digit_buffer.reserve(lead.len() + trail.len());
            // copy all leading characters into 'digits' string
            digit_buffer.push_str(lead);
            // copy all trailing characters after '.' into the digits string
            digit_buffer.push_str(trail);

            // count number of trailing digits
            let trail_digits = trail.chars().filter(|c| *c != '_').count();

            (digit_buffer.as_str(), trail_digits as i128)
        }
    };

    let scale = decimal_offset
        .checked_sub(exponent_value)
        .and_then(|scale| scale.checked_sub(unit.decimals() as i128))
        .and_then(|scale| {
            if scale > i64::MAX as i128 || scale < i64::MIN as i128 {
                None
            } else {
                Some(scale as i64)
            }
        })
        .ok_or_else(|| {
            ParseBalanceError::ExponentOverflow(format!(
                "Exponent overflow when parsing '{}'",
                value
            ))
        })?;

    let mut bignumber = U256::from_str_radix(digits, 10)
        .map_err(|err| ParseBalanceError::Other(err.to_string()))?;

    if scale > 18 || scale < -18 {
        return Err(ParseBalanceError::OverFlow(value.to_owned()));
    }

    if scale > 0 {
        bignumber = bignumber / U256::from(10usize).pow(scale as u32);
    } else {
        bignumber = bignumber * U256::from(10usize).pow(scale.abs() as u32);
    }

    Ok(Decimals(bignumber))
}

/// A Unit can be specified as a fixed decimals number,
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimals(U256);

impl FromStr for Decimals {
    type Err = ParseBalanceError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let unit_sep: &[_] = &['e', 'E'];

        let (base_part, unit) = match s.to_lowercase().find(unit_sep) {
            // exponent defaults to 0 if (e|E) not found
            None => return Err(ParseBalanceError::UnitIsEmpty),

            // split and parse exponent field
            Some(loc) => {
                // slice up to `loc` and 1 after to skip the 'e' char
                let (base, unit_part) = s.split_at(loc);

                let unit = Unit::from_str(unit_part)?;

                (base, unit)
            }
        };

        parse_dec_str(base_part, unit)
    }
}

impl From<Decimals> for U256 {
    fn from(value: Decimals) -> Self {
        value.0
    }
}

impl From<&Decimals> for U256 {
    fn from(value: &Decimals) -> Self {
        value.0
    }
}

impl Decimals {
    /// Parse string as `Decimal`
    pub fn parse<S: AsRef<str>, U: TryInto<Unit>>(value: S, unit: U) -> Result<Self>
    where
        U::Error: ToString,
    {
        let unit: Unit = unit
            .try_into()
            .map_err(|err| ParseBalanceError::ParseUnit(err.to_string()))?;

        parse_dec_str(value.as_ref(), unit)
    }

    pub fn format_unit(&self, unit: Unit) -> String {
        let result = format!("{}", self.0);

        let decimal_offset = result.len() as i64 - unit.decimals();

        if decimal_offset < 0 {
            "0.".to_string()
                + &"0".repeat(decimal_offset.abs() as usize)
                + &result[..result.len() - self.0.trailing_zeros() as usize]
                + " "
                + unit.to_string().as_str()
        } else {
            if unit.decimals() > self.0.trailing_zeros() as i64 {
                result[..decimal_offset as usize].to_owned()
                    + "."
                    + &result
                        [decimal_offset as usize..result.len() - self.0.trailing_zeros() as usize]
                    + " "
                    + unit.to_string().as_str()
            } else {
                result[..decimal_offset as usize].to_owned() + " " + unit.to_string().as_str()
            }
        }
    }
}

impl Display for Decimals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} wei", self.0)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_str() {
        assert_eq!(
            Decimals::parse("100.1", Unit::Ether).unwrap(),
            Decimals::parse("100.1", "ether").unwrap(),
        );

        assert_eq!(
            U256::from(Decimals::parse("100", Unit::Kwei).unwrap()),
            U256::from(100000u32)
        );

        assert_eq!(
            U256::from(Decimals::parse("100000", Unit::Kwei).unwrap()),
            U256::from(Decimals::parse("100", Unit::Mwei).unwrap()),
        );

        assert_eq!(
            U256::from(Decimals::parse("100111", Unit::Kwei).unwrap()),
            U256::from(Decimals::parse("100.111", Unit::Mwei).unwrap()),
        );

        assert_eq!(
            Decimals::parse("100.1", Unit::Kwei).unwrap().to_string(),
            "100100 wei"
        );

        assert_eq!(
            Decimals::parse("100100", Unit::Wei)
                .unwrap()
                .format_unit(Unit::Kwei),
            "100.1 kwei"
        );

        assert_eq!(
            Decimals::parse("00010", Unit::Wei)
                .unwrap()
                .format_unit(Unit::Kwei),
            "0.01 kwei"
        );

        assert_eq!(
            Decimals::parse("100001", Unit::Wei)
                .unwrap()
                .format_unit(Unit::Kwei),
            "100.001 kwei"
        );
    }
}
