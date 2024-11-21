use std::fmt::{self, Display};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{self, Error};

encoding_struct! {
    #[derive(Eq)]
    struct Location {
        registry: u8,
        code: u64,
        desc: &str,
    }
}

impl Location {
    pub fn custom(desc: &str) -> Self {
        Self::new(LocationRegistry::CustomNamed as u8, 0, desc)
    }

    pub fn is_custom(&self) -> bool {
        self.registry() == (LocationRegistry::CustomNamed as u8)
    }

    pub fn is_oktmo(&self) -> bool {
        self.registry() == (LocationRegistry::Oktmo as u8)
    }

    fn to_oktmo(&self) -> Result<Oktmo, Error> {
        if self.is_oktmo() {
            Oktmo(self.code()).verified()
        } else {
            Error::bad_location(&format!("can't convert to oktmo {:?}", self)).ok()
        }
    }

    pub fn covers(&self, other: &Location) -> Result<bool, Error> {
        if self.registry() != other.registry() {
            Error::bad_location("different registry types").ok()?
        }
        if self.is_oktmo() {
            let oktmo = other.to_oktmo()?;
            return self.to_oktmo().map(|v| v.covers(&oktmo));
        }
        if self.is_custom() {
            return Ok(true); // can't be checked automatically
        }
        Error::bad_location(&format!("can't match locations {:?} {:?}", self, other)).ok()
    }

    pub fn is_valid(&self) -> bool {
        match self.registry() {
            1 => true,
            2 => !self.desc().is_empty(),
            128 => !self.desc().is_empty(),
            _ => false,
        }
    }
}

impl Default for Location {
    fn default() -> Self {
        // todo!("Must be 'all'")
        Self::new(LocationRegistry::Oktmo as u8, 0, "")
    }
}

impl FromStr for Location {
    type Err = error::Error;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        if src.len() == 0 {
            Error::bad_location(src).ok()
        } else if let Some(oktmo) = src.strip_prefix("oktmo::") {
            let (code, desc) = if let Some(split) = oktmo.find("::") {
                (&oktmo[..split], Some(&oktmo[split + 2..]))
            } else {
                (oktmo, None)
            };

            let oktmo = code
                .parse::<u64>()
                .map_err(|_| Error::bad_location(src))
                .and_then(|v| Oktmo::new(v).verified());

            if let Some(desc) = desc {
                oktmo.map(|oktmo| oktmo.extend(desc))
            } else {
                oktmo.map(Into::into)
            }
        } else {
            Ok(Self::custom(src))
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.registry() {
            1 => write!(f, "oktmo::{}", self.code()),
            2 => write!(f, "oktmo::{}::{}", self.code(), self.desc()),
            128 => write!(f, "{}", self.desc()),
            // FIXME
            _ => todo!("Display for invalid Location is not implemented"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LocationRegistry {
    Undefined = 0,
    Oktmo = 1,
    OktmoExtended = 2,
    CustomNamed = 128,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Oktmo(u64);

impl Oktmo {
    fn new(code: u64) -> Self {
        Oktmo(code)
    }

    fn extend(self, extension: &str) -> Location {
        Location::new(LocationRegistry::OktmoExtended as u8, self.0, extension)
    }

    fn verified(self) -> Result<Self, Error> {
        Ok(self) // TODO verify value
    }

    fn covers(&self, other: &Self) -> bool {
        other.0.to_string().starts_with(&self.0.to_string())
    }
}

impl From<Oktmo> for Location {
    fn from(src: Oktmo) -> Self {
        Self::new(LocationRegistry::Oktmo as u8, src.0, "")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn oktmo_parsing() {
        let locaton_str = "oktmo::45379000";
        let location = Location::from_str(locaton_str).expect("Unable to parse valid location");
        let true_location = Location::new(LocationRegistry::Oktmo as u8, 45379000, "");
        assert_eq!(location, true_location)
    }

    #[test]
    fn oktmo_extended_parsing() {
        let locaton_str = "oktmo::45379000::Проспект Мира";
        let location = Location::from_str(locaton_str).expect("Unable to parse valid location");
        let true_location = Location::new(
            LocationRegistry::OktmoExtended as u8,
            45379000,
            "Проспект Мира",
        );
        assert_eq!(location, true_location);

        let locaton_str = "oktmo::45379000::Проспект Мира :: дом 6";
        let location = Location::from_str(locaton_str).expect("Unable to parse valid location");
        let true_location = Location::new(
            LocationRegistry::OktmoExtended as u8,
            45379000,
            "Проспект Мира :: дом 6",
        );
        assert_eq!(location, true_location);

        let locaton_str = "oktmo::45379000::Проспект Мира::";
        let location = Location::from_str(locaton_str).expect("Unable to parse valid location");
        let true_location = Location::new(
            LocationRegistry::OktmoExtended as u8,
            45379000,
            "Проспект Мира::",
        );
        assert_eq!(location, true_location);
    }
}
