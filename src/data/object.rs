use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use serde_plain;

use blockp_core::crypto::{self, Hash};

use crate::error::Error;

pub type ObjectId = Hash;

#[repr(u8)]
#[derive(PartialEq, Serialize, Deserialize, TryFromPrimitive)]
#[serde(rename_all = "snake_case")]
pub enum ObjectType {
    Undefined = 0,
    Trademark = 1,
    WellknownTrademark = 2,
    AppellationOfOrigin = 3,
    AppellationOfOriginRights = 4,
    Pharmaceutical = 5,
    Invention = 6,
    UtilityModel = 7,
    IndustrialModel = 8,
    Tims = 9,
    Program = 10,
    Database = 11,
    GeographicalIndication = 12,
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_plain::to_string::<ObjectType>(&self).unwrap()
        )
    }
}

encoding_struct! {
    #[derive(Eq)]
    struct ObjectIdentity {
        class: u8,
        reg_number: &str
    }
}

impl Display for ObjectIdentity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let class = ObjectType::try_from(self.class()).unwrap(); //TODO
        write!(f, "{}::{}", class, self.reg_number())
    }
}

impl ObjectIdentity {
    pub fn id(&self) -> ObjectId {
        crypto::hash(&self.to_string().as_bytes())
    }

    pub fn is_valid(&self) -> bool {
        match self.class() {
            0..=12 => {}
            _ => return false,
        };

        let valid_id = if self.class() == ObjectType::AppellationOfOrigin as u8 {
            self.is_valid_appellation_of_origin()
        } else if self.class() == ObjectType::AppellationOfOriginRights as u8 {
            self.is_valid_appellation_of_origin_rights()
        } else {
            self.is_valid_reg_number()
        };

        valid_id
    }

    pub fn is_sellable(&self) -> bool {
        self.class() != ObjectType::AppellationOfOrigin as u8
            && self.class() != ObjectType::AppellationOfOriginRights as u8
            && self.class() != ObjectType::GeographicalIndication as u8
    }

    pub fn is_trademark(&self) -> bool {
        self.class() == ObjectType::Trademark as u8
    }

    pub fn is_appellation_of_origin(&self) -> bool {
        self.class() == ObjectType::AppellationOfOrigin as u8
    }

    fn is_valid_reg_number(&self) -> bool {
        let reg_number_chars = self.reg_number().chars().count();
        reg_number_chars <= 20
            && reg_number_chars > 0
            && self.reg_number().chars().all(|c| {
                matches!(
                    c,
                    '0'..='9' | '-' | '_'
                    | 'a'..='z' | 'A'..='Z'
                    | 'а'..='я' | 'ё'
                    | 'А'..='Я' | 'Ё'
                )
            })
    }

    fn is_numeric_sequence(sequence: &str) -> bool {
        let number_chars = sequence.chars().count();
        number_chars < 256 // possibly unlimited
            && number_chars > 0
            && sequence.chars().all(|c| {
            matches!(c, '0'..='9')
        })
    }

    fn is_valid_appellation_of_origin_str(object: &str) -> bool {
        if object == "0" {
            return false;
        }
        Self::is_numeric_sequence(object)
    }

    fn is_valid_appellation_of_origin(&self) -> bool {
        Self::is_valid_appellation_of_origin_str(self.reg_number())
    }

    fn is_valid_appellation_of_origin_rights(&self) -> bool {
        let mut valid = true;
        let split: Vec<&str> = self.reg_number().split("/").collect();

        if split.len() != 2 {
            return false;
        }
        valid = valid && Self::is_valid_appellation_of_origin_str(split[0]);
        valid = valid && Self::is_numeric_sequence(split[1]);

        valid
    }

    fn new_verified(class: ObjectType, id: &str) -> Result<ObjectIdentity, Error> {
        let object = ObjectIdentity::new(class as u8, id);

        if object.is_valid() {
            Ok(object)
        } else {
            Error::bad_object_format(&object.to_string(), "invalid number").ok()
        }
    }
}

impl FromStr for ObjectIdentity {
    type Err = Error;

    fn from_str(object: &str) -> Result<Self, Self::Err> {
        let parts = object.split("::").collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Error::bad_object_format(object, "invalid format").ok();
        }
        let object_type = serde_plain::from_str::<ObjectType>(parts[0])
            .map_err(|_| Error::bad_object_format(object, "invalid object type"))?;
        let id = parts[1];

        ObjectIdentity::new_verified(object_type, id)
    }
}

encoding_struct! {
    struct Change {
        tx_hash: &Hash,
    }
}
