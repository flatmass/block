use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

use blockp_core::crypto::{self, Hash};
use blockp_core::storage::StorageKey;

use crate::error::Error;

pub type MemberId = Hash;

#[repr(u8)]
#[derive(PartialEq)]
pub enum MemberType {
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
    #[derive(Eq, Hash)]
    struct MemberIdentity {
        class: u8,
        number: &str,
    }
}

impl StorageKey for MemberIdentity {
    /// 1 byte for 'class' + length of 'number'
    fn size(&self) -> usize {
        1 + self.number().len()
    }

    fn write(&self, buffer: &mut [u8]) {
        self.class().write(&mut buffer[0..1]);
        self.number().write(&mut buffer[1..]);
    }

    fn read(buffer: &[u8]) -> Self::Owned {
        let class = buffer[0];
        let number = unsafe { std::str::from_utf8_unchecked(&buffer[1..]) };
        Self::new(class, number)
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

    #[allow(unreachable_code)]
    pub fn is_valid(&self) -> bool {
        #[cfg(feature = "disable_member_identity_validation")]
        return true;

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

encoding_struct! {
    #[derive(Eq, Hash)]
    struct MemberEsiaToken {
        token: &str,
        oid: &str,
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use blockp_core::storage::StorageKey;

    use super::*;

    #[test]
    fn storage_key_member_identity() {
        let members = [
            MemberIdentity::from_str("ogrn::1053600591197").unwrap(),
            MemberIdentity::new(0, ""),
            MemberIdentity::new(0, "asdfjasdjfkj23904u9fjoadjfojf2940jufojadfjaspofjoaasdfjasdjfkj23904u9fjoadjfojf2940jufojadfjaspofjoaasdfjasdjfkj23904u9fjoadjfojf2940jufojadfjaspofjoaasdfjasdjfkj23904u9fjoadjfojf2940jufojadfjaspofjoaasdfjasdjfkj23904u9fjoadjfojf2940jufojadfjaspofjoaasdfjasdjfkj23904u9fjoadjfojf2940jufojadfjaspofjoa"),
        ];

        assert_round_trip_eq(&members);
    }

    fn assert_round_trip_eq<T>(values: &[T])
    where
        T: StorageKey + PartialEq<<T as ToOwned>::Owned> + Debug,
        <T as ToOwned>::Owned: Debug,
    {
        for original_value in values.iter() {
            let mut buffer = get_buffer(original_value);
            original_value.write(&mut buffer);
            let new_value = <T as StorageKey>::read(&buffer);
            assert_eq!(*original_value, new_value);
        }
    }

    fn get_buffer<T: StorageKey + ?Sized>(key: &T) -> Vec<u8> {
        vec![0; key.size()]
    }

    #[test]
    fn de_member_identity() {
        let data = "snils::invalid member";
        let member = MemberIdentity::from_str(data);

        if cfg!(feature = "disable_member_identity_validation") {
            member.unwrap();
        } else {
            println!("{}", member.unwrap_err());
        };
    }
}
