use super::classifier::Classifier;
use super::conditions::ContractType;
use super::location::Location;
use super::member::MemberIdentity;
use super::object::{ObjectIdentity, ObjectType};
use super::time::{Specification, Term};
use crate::error::Error;
use chrono::{DateTime, Utc};
use std::convert::TryInto;

use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

encoding_struct! {
    struct Ownership {
        rightholder: MemberIdentity,
        contract_type: u8,
        exclusive: bool,
        distribution: u8,
        location: Vec<Location>,
        classifiers: Vec<Classifier>,
        starting_time: DateTime<Utc>,
        expiration_time: Option<DateTime<Utc>>,
    }
}

impl Ownership {
    pub fn rights(&self) -> Rights {
        let mut flags = Flag::empty();
        if self.exclusive() {
            flags.set(Flag::EXCLUSIVE, true);
        }
        if self.contract_type() == 0 {
            flags.set(Flag::OWNER, true);
        }
        if !self.classifiers().is_empty() {
            flags.set(Flag::CLASSIFIED, true);
        }
        match Distribution::try_from(self.distribution()) {
            Ok(Distribution::Able) => flags.set(Flag::CAN_DISTRIBUTE, true),
            Ok(Distribution::WithWrittenPermission) => {
                flags.set(Flag::DISTRIBUTE_WITH_WRITTEN_PERMISSION, true)
            }
            Ok(Distribution::Unable) => (),
            Err(_) => (),
        }
        if self.expiration_time().is_none() {
            flags.set(Flag::NO_EXPIRATION_TIME, true);
        }
        Rights::new(
            flags.bits(),
            self.contract_type(),
            self.location(),
            self.classifiers(),
            self.starting_time(),
            self.expiration_time(),
        )
    }
}

encoding_struct! {
    struct OwnershipUnstructured {
        data: &str,
        rightholder: Option<MemberIdentity>,
        exclusive: Option<bool>,
    }
}

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, TryFromPrimitive, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Distribution {
    Able = 1,
    WithWrittenPermission = 2,
    Unable = 3,
}

encoding_struct! {
    struct Rights {
        flags: u16,
        contract_type: u8,
        location: Vec<Location>,
        classifiers: Vec<Classifier>,
        starting_time: DateTime<Utc>,
        expiration_time: Option<DateTime<Utc>>,
    }
}

impl Rights {
    pub fn new_owned() -> Rights {
        let flags = Flag::EXCLUSIVE | Flag::CAN_DISTRIBUTE | Flag::OWNER;
        let contract_type = ContractType::Undefined as u8;
        Rights::new(
            flags.bits(),
            contract_type,
            vec![Location::default()],
            vec![],
            Utc::now(),
            None,
        )
    }

    pub fn check_term(&self, object: &ObjectIdentity, term: Term) -> Result<i8, Error> {
        use ObjectType::*;
        let default_duration = match object
            .class()
            .try_into()
            .map_err(|_| Error::internal_bad_struct("ObjectIdentity"))?
        {
            Trademark | WellknownTrademark | AppellationOfOrigin | AppellationOfOriginRights => {
                return Ok(1)
            }
            Pharmaceutical => todo!(),
            Invention => chrono::Duration::days(20 * 365),
            UtilityModel => chrono::Duration::days(10 * 365),
            IndustrialModel => chrono::Duration::days(5 * 365),
            Undefined => return Err(Error::internal_bad_struct("ObjectIdentity")),
        };
        let expiration_time = self
            .expiration_time()
            .unwrap_or(self.starting_time() + default_duration);

        match term
            .specification()
            .try_into()
            .map_err(|_| Error::internal_bad_struct("Term"))?
        {
            Specification::For => Ok(0),
            Specification::To | Specification::Until if term.date().is_some() => {
                if term.date().unwrap() > expiration_time {
                    Ok(-1)
                } else {
                    Ok(1)
                }
            }
            Specification::Forever => Ok(1),
            Specification::To | Specification::Until => Err(Error::internal_bad_struct("Term"))?,
        }
    }

    pub fn is_owner(&self) -> bool {
        self.has(Flag::OWNER)
    }

    fn has(&self, f: Flag) -> bool {
        Flag::from_bits(self.flags())
            .unwrap_or(Flag::UNDEFINED)
            .contains(f)
    }
}

bitflags! {
    struct Flag: u16 {
        const UNDEFINED = 0;
        const EXCLUSIVE = 1;
        const CLASSIFIED = 4;
        const CAN_DISTRIBUTE = 8;
        const DISTRIBUTE_WITH_WRITTEN_PERMISSION = 16;
        const NO_EXPIRATION_TIME = 32;
        const OWNER = 128;
    }
}