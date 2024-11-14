use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use num_enum::TryFromPrimitive;
use serde_derive::{Deserialize, Serialize};

use crate::error::Error;

encoding_struct! {
    #[derive(Eq)]
    struct Classifier {
        registry: u8,
        value: &str
    }
}

impl Default for Classifier {
    fn default() -> Self {
        Classifier::new(ClassifierRegistry::All as u8, "")
    }
}

impl Display for Classifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let registry = ClassifierRegistry::try_from(self.registry())
            .expect("Classifier registry value is invalid");
        let registry =
            serde_plain::to_string(&registry).expect("Classifier registry value is invalid");
        write!(f, "{}::{}", registry, self.value())
    }
}

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, TryFromPrimitive)]
#[serde(rename_all = "lowercase")]
pub enum ClassifierRegistry {
    All = 0,
    Mktu = 1,
    Mpk = 2,
    Spk = 3,
    Mkpo = 4,
}

impl FromStr for Classifier {
    type Err = Error;

    fn from_str(c: &str) -> Result<Self, Self::Err> {
        let parts = c.split("::").collect::<Vec<&str>>();
        let reg = parts
            .get(0)
            .and_then(|reg_str| serde_plain::from_str::<ClassifierRegistry>(reg_str).ok())
            .ok_or_else(|| Error::bad_classifier_format(c))?;

        if reg == ClassifierRegistry::All && parts.len() == 1 {
            Ok(Classifier::new(reg as u8, ""))
        } else if reg != ClassifierRegistry::All && parts.len() == 2 {
            Ok(Classifier::new(reg as u8, parts[1]))
        } else {
            Error::bad_classifier_format(c).ok()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Mktu(u64);

impl Into<u64> for Mktu {
    fn into(self) -> u64 {
        self.0
    }
}
