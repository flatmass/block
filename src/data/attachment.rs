use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use blockp_core::crypto::{self, Hash};

use crate::data::strings::{verify_filename, verify_str};
use crate::error::{self, Error};

pub type DocumentId = Hash;

encoding_struct! {
    struct File {
        name: &str,
        desc: &str,
        hash: &Hash,
    }
}

encoding_struct! {
    #[derive(Eq)]
    struct Sign {
        data: &[u8],
    }
}

impl Sign {
    pub fn new_verified(blob: &[u8]) -> error::Result<Self> {
        let sign = Self::new(blob);

        #[cfg(not(feature = "disable_sign_checks"))]
        sign.verify()?;

        Ok(sign)
    }

    fn decode_base64_detached_sign(blob: &[u8]) -> error::Result<Vec<u8>> {
        std::str::from_utf8(blob)
            .map_err(Error::from)
            .and_then(|sign_base64| {
                // TODO: sign_base64.fold()
                let mut res = String::new();
                for l in sign_base64.lines() {
                    if !l.contains("-BEGIN CMS-") && !l.contains("-END CMS-") {
                        res.push_str(l);
                    }
                }
                debug!("{:?}", res);
                base64::decode(res.as_bytes()).map_err(Error::from)
            })
    }

    pub fn verify(&self) -> error::Result<()> {
        crypto::validate_detached_sign(self.data()).map_err(|e| {
            warn!("Failed to validate detached signature {:?}", e);
            Error::from(e)
        })
    }

    pub fn verify_data(&self, _file: &[u8]) -> error::Result<()> {
        #[cfg(not(feature = "disable_sign_checks"))]
        crypto::verify_detached_sign(_file, self.data()).map_err(Into::into);
        Ok(())
    }

    #[allow(unused)]
    pub fn verify_hash(&self, hash: &Hash) -> error::Result<()> {
        crypto::verify_detached_sign_with_hash(hash.as_ref(), self.data())
            .map_err(error::Error::from)
    }
}

impl FromStr for Sign {
    type Err = Error;

    fn from_str(src: &str) -> error::Result<Self> {
        let blob = src.as_bytes();
        if let Ok(sign) = Self::decode_base64_detached_sign(blob) {
            Self::new_verified(&sign)
        } else {
            Self::new_verified(blob)
        }
    }
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

encoding_struct! {
    struct AttachmentMetadata {
        name: &str,
        description: Option<String>,
        file_type: u8,
        timestamp: DateTime<Utc>,
    }
}

encoding_struct! {
    struct AttachmentMetadataWithHash {
        metadata: AttachmentMetadata,
        tx_hash: &Hash,
    }
}

encoding_struct! {
    struct Attachment {
        metadata: AttachmentMetadata,
        data: &[u8],
        sign: Option<Sign>

    }
}

impl Attachment {
    pub fn verify(&self) -> Result<(), Error> {
        verify_filename(self.metadata().name())?;
        if let Some(data) = self.metadata().description() {
            verify_str(data.as_ref(), "description")?;
        };
        self.sign().map(|sign| sign.verify()).transpose()?;
        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentType {
    Other = 0,
    Deed = 1,
    Application = 2,
    Notification = 3,
}

impl FromStr for AttachmentType {
    type Err = Error;

    fn from_str(src: &str) -> error::Result<Self> {
        serde_plain::from_str(src).map_err(|_| Error::bad_file_type(src))
    }
}

impl TryFrom<u8> for AttachmentType {
    type Error = Error;

    fn try_from(val: u8) -> error::Result<Self> {
        let result = match val {
            0 => AttachmentType::Other,
            1 => AttachmentType::Deed,
            2 => AttachmentType::Application,
            3 => AttachmentType::Notification,
            _ => return Err(Error::unexpected_file_type()),
        };
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn de_attachment_type() {
        let strings = vec!["other", "deed", "application", "notification"];
        let att_tpye = vec![
            AttachmentType::Other,
            AttachmentType::Deed,
            AttachmentType::Application,
            AttachmentType::Notification,
        ];
        strings
            .into_iter()
            .zip(att_tpye)
            .for_each(|(val, expected)| {
                let received = AttachmentType::from_str(val).unwrap();
                assert_eq!(received, expected);
            });
    }

    #[test]
    fn de_sign() {
        let data = "invalid signature";
        let sign = Sign::from_str(data);

        if cfg!(feature = "disable_sign_checks") {
            sign.unwrap();
        } else {
            sign.unwrap_err();
        };
    }

    #[test]
    fn display_sign() {
        let data = "invalid signature";
        let sign = Sign::from_str(data);

        if cfg!(feature = "disable_sign_checks") {
            sign.unwrap();
        } else {
            sign.unwrap_err();
        };
    }
}
