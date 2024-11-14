use crate::error::Error;
use blockp_core::crypto::{self, Hash};
use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

pub type MemberId = Hash;

#[repr(u8)]
#[derive(PartialEq)]
enum MemberType {
    Ogrn = 0,
    Ogrnip = 1,
    Snils = 2,
}

impl TryFrom<u8> for MemberType {
    type Error = u8;

    fn try_from(num: u8) -> Result<MemberType, Self::Error> {
        match num {
            0 => Ok(MemberType::Ogrn),
            1 => Ok(MemberType::Ogrnip),
            2 => Ok(MemberType::Snils),
            _ => Err(num),
        }
    }
}

impl fmt::Display for MemberType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MemberType::Ogrn => write!(f, "ogrn"),
            MemberType::Ogrnip => write!(f, "ogrnip"),
            MemberType::Snils => write!(f, "snils"),
        }
    }
}

encoding_struct! {
    #[derive(Eq)]
    struct MemberIdentity {
        class: u8,
        number: &str,
    }
}

impl MemberIdentity {
    pub fn id(&self) -> MemberId {
        crypto::HashStream::new()
            .update(&[self.class()])
            .update(self.number().as_bytes())
            .hash()
    }

    pub fn is_legal_entity(&self) -> bool {
        MemberType::try_from(self.class()) == Ok(MemberType::Ogrn)
    }

    pub fn is_entrepreneur(&self) -> bool {
        MemberType::try_from(self.class()) == Ok(MemberType::Ogrnip)
    }

    pub fn is_person(&self) -> bool {
        MemberType::try_from(self.class()) == Ok(MemberType::Snils)
    }

    pub fn is_valid(&self) -> bool {
        match self.class() {
            0 => self.is_valid_ogrn(),
            1 => self.is_valid_ogrnip(),
            2 => self.is_valid_snils(),
            _ => false,
        }
    }

    fn is_valid_ogrn(&self) -> bool {
        Some(self.number())
            .filter(|ogrn_str| ogrn_str.chars().count() == 13)
            .filter(|ogrn_str| {
                let kind = ogrn_str.chars().nth(0).unwrap();
                kind == '1' || kind == '5'
            })
            .and_then(|ogrn_str| ogrn_str.parse::<u64>().ok())
            .filter(|ogrn| {
                let base = ogrn / 10;
                let control = ogrn % 10;
                let calculated = base % 11 % 10;
                control == calculated
            })
            .is_some()
    }

    fn is_valid_ogrnip(&self) -> bool {
        Some(self.number())
            .filter(|ogrnip_str| ogrnip_str.chars().count() == 15)
            .filter(|ogrnip_str| {
                let kind = ogrnip_str.chars().nth(0).unwrap();
                kind == '3'
            })
            .and_then(|ogrnip_str| ogrnip_str.parse::<u64>().ok())
            .filter(|ogrnip| {
                let base = ogrnip / 10;
                let control = ogrnip % 10;
                let calculated = base % 13 % 10;
                control == calculated
            })
            .is_some()
    }

    fn is_valid_snils(&self) -> bool {
        Some(self.number())
            .filter(|snils_str| snils_str.chars().count() == 11)
            .and_then(|snils_str| snils_str.parse::<u64>().ok())
            .filter(|snils| {
                let base = snils / 100;
                let control = snils % 100;
                let calculated = (1..=9)
                    .fold(0, |acc, i| acc + base / 10u64.pow(i - 1) % 10 * i as u64)
                    % 101
                    % 100;
                control == calculated
            })
            .is_some()
    }
}

impl FromStr for MemberIdentity {
    type Err = Error;

    fn from_str(member_str: &str) -> Result<Self, Self::Err> {
        let parts = member_str.split("::").collect::<Vec<&str>>();
        match parts.as_slice() {
            &["ogrn", id] => Ok(MemberIdentity::new(MemberType::Ogrn as u8, id)),
            &["ogrnip", id] => Ok(MemberIdentity::new(MemberType::Ogrnip as u8, id)),
            &["snils", id] => Ok(MemberIdentity::new(MemberType::Snils as u8, id)),
            _ => Error::bad_member_format(member_str).ok(),
        }
        .and_then(|identity| {
            if identity.is_valid() {
                Ok(identity)
            } else {
                Error::bad_member_format(member_str).ok()
            }
        })
    }
}

impl fmt::Display for MemberIdentity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let member_type = MemberType::try_from(self.class()).expect("Bad MemberType");
        write!(f, "{}::{}", member_type, self.number())
    }
}
