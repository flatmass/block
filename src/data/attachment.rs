use std::convert::TryFrom;
use std::fmt;
use std::str::FromStr;

use blockp_core::crypto::{self, Hash};

use crate::data::strings::verify_filename;
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

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

encoding_struct! {
    struct Attachment {
        name: &str,
        data: &[u8],
        file_type: u8,
    }
}

encoding_struct! {
    struct SignedAttachment {
        file: Attachment,
        sign: Sign,
    }
}

impl Sign {
    pub fn new_verified(blob: &[u8]) -> error::Result<Self> {
        let sign = Self::new(blob);
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

    pub fn verify_data(&self, file: &[u8]) -> error::Result<()> {
        crypto::verify_detached_sign(file, self.data()).map_err(Into::into)
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
            Ok(Self::new(&sign))
        } else {
            Ok(Self::new(blob))
        }
    }
}

impl Attachment {
    pub fn verify(&self) -> Result<(), Error> {
        verify_filename(self.name())
    }
}

impl SignedAttachment {
    pub fn verify(&self) -> Result<(), Error> {
        self.file().verify()?;
        // self.sign().verify()?;
        // self.sign().verify_data(self.file().data())
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
            _ => return Err(Error::unexpected_file_type()),
        };
        Ok(result)
    }
}
