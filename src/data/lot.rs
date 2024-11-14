use std::convert::TryFrom;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

use blockp_core::crypto::Hash;

use crate::error::{self, Error};

pub type LotId = Hash;

pub fn verify_lot_name(name: &str) -> error::Result<()> {
    if name.len() <= 256 {
        Ok(())
    } else {
        Err(Error::too_long_param("name"))
    }
}

pub fn verify_lot_desc(desc: &str) -> error::Result<()> {
    // 10KB
    if desc.len() <= 10240 {
        Ok(())
    } else {
        Err(Error::too_long_param("desc"))
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, TryFromPrimitive)]
#[serde(rename_all = "snake_case")]
pub enum SaleType {
    Auction = 1,
    PrivateSale,
}

impl FromStr for SaleType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str(s).map_err(|_| Error::bad_sale_type(s))
    }
}

encoding_struct! {
    struct Lot {
        name: &str,
        desc: &str,
        price: u64,
        sale_type: u8,
        opening_time: DateTime<Utc>,
        closing_time: DateTime<Utc>,
    }
}

impl Lot {
    pub fn verify(&self) -> error::Result<&Self> {
        verify_lot_name(self.name())?;
        verify_lot_desc(self.desc())?;

        if self.opening_time() >= self.closing_time() {
            Error::bad_time_period(self.opening_time(), self.closing_time()).ok()
        } else {
            Ok(self)
        }
    }

    pub fn is_auction(&self) -> bool {
        SaleType::try_from(self.sale_type()) == Ok(SaleType::Auction)
    }

    pub fn is_private_sale(&self) -> bool {
        SaleType::try_from(self.sale_type()) == Ok(SaleType::PrivateSale)
    }
}

encoding_struct! {
    struct Bid {
        value: u64,
    }
}

encoding_struct! {
    struct LotState {
        name: &str,
        price: u64,
        status: u8,
    }
}

impl LotState {
    pub fn open(name: &str, price: u64) -> Self {
        LotState::new(name, price, LotStatus::New as u8)
    }

    pub fn set_price(self, price: u64) -> Self {
        LotState::new(self.name(), price, self.status())
    }

    pub fn set_status(self, status: LotStatus) -> Self {
        LotState::new(self.name(), self.price(), status as u8)
    }

    pub fn is_new(&self) -> bool {
        self.status() == (LotStatus::New as u8)
    }

    pub fn is_verified(&self) -> bool {
        self.status() == (LotStatus::Verified as u8)
    }

    pub fn is_completed(&self) -> bool {
        self.status() == (LotStatus::Completed as u8)
    }

    pub fn is_executed(&self) -> bool {
        self.status() == (LotStatus::Executed as u8)
    }

    pub fn is_closed(&self) -> bool {
        self.status() == (LotStatus::Closed as u8)
    }
}

#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, TryFromPrimitive)]
#[serde(rename_all = "lowercase")]
pub enum LotStatus {
    New = 0,         // after creation
    Rejected = 1,    // after internal verification (bad)
    Verified = 2,    // after internal verification (good)
    Completed = 3,   // after lot timeout
    Executed = 4,    // after publishing bids
    Closed = 5,      // after lot execution while object is updating
    Undefined = 255, // something has been changed with objects while lot was opened
}

impl Default for LotStatus {
    fn default() -> Self {
        Self::Undefined
    }
}

impl FromStr for LotStatus {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str(s).map_err(|_| Error::bad_lot_status(s))
    }
}
