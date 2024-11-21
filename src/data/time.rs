use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use crate::error::Error;

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, TryFromPrimitive)]
#[serde(rename_all = "snake_case")]
pub enum Specification {
    For = 1,
    To = 2,
    Until = 3,
    Forever = 4,
}

encoding_struct! {
    #[derive(Eq)]
    struct Duration {
        months: u16,
        days: u16,
    }
}

impl FromStr for Duration {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split(":").collect::<Vec<&str>>();

        if parts.len() != 2 {
            return Error::bad_term_format(s).ok();
        }
        let months = parts[0].parse::<u16>()?;
        let days = parts[1].parse::<u16>()?;
        Ok(Self::new(months, days))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.months(), self.days())
    }
}

encoding_struct! {
    struct Term {
        specification: u8,
        duration: Option<Duration>,
        date: Option<DateTime<Utc>>,
    }
}
