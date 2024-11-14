use std::fmt;

use actix_web::{HttpResponse, ResponseError};
use chrono::{DateTime, Utc};
use futures::IntoFuture;
use serde::ser::Serialize;
use serde::Serializer;

use blockp_core::api::backends::actix::FutureResponse;
use blockp_core::crypto::Hash;

use crate::data::attachment::DocumentId;
use crate::data::conditions::{Check, Conditions};
use crate::data::contract::{ContractId, ContractStatus};
use crate::data::lot::LotId;
use crate::data::member::MemberIdentity;
use crate::data::object::ObjectIdentity;
use crate::response;
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

#[repr(u8)]
#[derive(Debug, Fail, Clone, Copy, Serialize, TryFromPrimitive)]
enum Code {
    #[fail(display = "Data wasn't found")]
    NotFound = 1,

    #[fail(display = "Unexpected data")]
    Unexpected = 2,

    #[fail(display = "Data already exists")]
    AlreadyExists = 3,

    #[fail(display = "Permission denied")]
    PermissionDenied = 4,

    #[fail(display = "Crypto error occurs")]
    Crypto = 5,

    #[fail(display = "Bad state")]
    BadState = 6,

    #[fail(display = "Bad parameter")]
    BadParam = 7,

    #[fail(display = "Bad argument value")]
    BadValue = 8,

    #[fail(display = "Internal")]
    Internal = 9,

    #[fail(display = "Check failed")]
    CheckFail = 10,

    #[fail(display = "Other error")]
    Other = 255, // do not use in transactions
}

impl<T> Into<Result<T>> for Code {
    fn into(self) -> Result<T> {
        Err(Error::from(self))
    }
}

#[derive(Debug, Fail, Clone)]
pub struct Error {
    code: Code,
    desc: String,
}

impl Error {
    pub fn participant_already_exists(member: &MemberIdentity) -> Self {
        let desc = format!("identical participant already exists '{}'", member);
        Error::with_info(Code::AlreadyExists, desc)
    }

    pub fn no_attachment(doc_tx_hash: &DocumentId) -> Self {
        let desc = format!("attachment wasn't found '{}'", doc_tx_hash);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn duplicate_sign() -> Self {
        Error::with_info(Code::AlreadyExists, "duplicate signature".to_owned())
    }

    pub fn duplicate_payment() -> Self {
        Error::with_info(
            Code::AlreadyExists,
            "payment with the same payment number exists".to_owned(),
        )
    }

    pub fn no_object(object: &ObjectIdentity) -> Self {
        let desc = format!("object '{}' wasn't found", object);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn no_owner(object: &ObjectIdentity) -> Self {
        let desc = format!("object owner wasn't found '{}'", object);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn no_transaction(tx_hash: &Hash) -> Self {
        let desc = format!("transaction wasn't found '{}'", tx_hash);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn unexpected_tx_type(tx_hash: &Hash) -> Self {
        let desc = format!("unexpected transaction type '{}'", tx_hash);
        Error::with_info(Code::Unexpected, desc)
    }

    pub fn no_lot(lot_id: &LotId) -> Self {
        let desc = format!("lot wasn't found '{}'", lot_id);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn duplicate_lot(lot_id: &LotId) -> Self {
        let desc = format!("lot already exists '{}'", lot_id);
        Error::with_info(Code::AlreadyExists, desc)
    }

    pub fn no_permissions() -> Self {
        Error::with_info(
            Code::PermissionDenied,
            "no permissions to do this action".to_owned(),
        )
    }

    pub fn bad_state(desc: &str) -> Self {
        Error::with_info(Code::BadState, desc.to_owned())
    }

    pub fn bad_lot_status(status: &str) -> Self {
        let desc = format!("bad lot status '{}'", status);
        Error::with_info(Code::BadState, desc)
    }

    pub fn bad_datetime_format(datetime: &str) -> Self {
        let desc = format!("datetime doesn't match RFC-3339 '{}'", datetime);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_price_format(price: &str) -> Self {
        let desc = format!("bad price format '{}'", price);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn duplicate_object(object: &ObjectIdentity) -> Self {
        let desc = format!("duplicate objects '{}'", object);
        Error::with_info(Code::AlreadyExists, desc)
    }

    pub fn locked_object(object: &ObjectIdentity) -> Self {
        let desc = format!("object is locked '{}'", object);
        Error::with_info(Code::PermissionDenied, desc)
    }

    pub fn action_refused(info: &str) -> Self {
        let desc = format!("action refused: '{}'", info);
        Error::with_info(Code::PermissionDenied, desc)
    }

    pub fn bad_object_format(object: &str) -> Self {
        let desc = format!("bad object identity '{}'", object);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_term_format(term: &str) -> Self {
        let desc = format!("bad term '{}'", term);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_member_format(member: &str) -> Self {
        let desc = format!("bad member identity '{}'", member);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_contract_type_format(contract_type: &str) -> Self {
        let desc = format!("bad contract type '{}'", contract_type);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_classifier_format(classifier: &str) -> Self {
        let desc = format!("bad classifier '{}'", classifier);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_json(src: &str, e: serde_json::Error) -> Self {
        let desc = format!("bad json: \"{}\"\n\n{}", src, e);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_time_period(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        let desc = format!(
            "bad time period '{}' - '{}'",
            start.to_string(),
            end.to_string()
        );
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_lot_time_extension() -> Self {
        let desc = format!("can't set lot's closing time earlier than it was");
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_signature(msg: &str) -> Self {
        let desc = format!("bad signature: '{}'", msg);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn out_of_time(datetime: DateTime<Utc>) -> Self {
        let desc = format!("out of time '{}'", datetime.to_string());
        Error::with_info(Code::BadValue, desc)
    }

    pub fn no_time_provider() -> Self {
        let desc = "no time provider, can't retrive the current time";
        Error::with_info(Code::NotFound, desc.to_owned())
    }

    pub fn missed_bid(tx_hash: &Hash) -> Self {
        let desc = format!("missed bid '{}'", tx_hash);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn no_private_data(tx_hash: &Hash) -> Self {
        let desc = format!(
            "linked private transaction or its data weren't found '{}'",
            tx_hash
        );
        Error::with_info(Code::NotFound, desc)
    }

    pub fn no_param(name: &str) -> Self {
        let desc = format!("parameter '{}' wasn't found", name);
        Error::with_info(Code::BadParam, desc)
    }

    pub fn empty_param(name: &str) -> Self {
        let desc = format!("parameter '{}' is required, value is empty", name);
        Error::with_info(Code::BadParam, desc)
    }

    pub fn empty_transaction_param(name: &str) -> Self {
        let desc = format!("parameter '{}' is required, value is empty", name);
        Error::with_info(Code::BadState, desc)
    }

    pub fn too_long_param(name: &str) -> Self {
        let desc = format!("parameter '{}' is too long", name);
        Error::with_info(Code::BadParam, desc)
    }

    pub fn unexpected_param_value(name: &str) -> Self {
        let desc = format!("parameter '{}' has unexpected value", name);
        Error::with_info(Code::BadParam, desc)
    }

    pub fn bad_location(info: &str) -> Self {
        let desc = format!("bad location: '{}'", info);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_sale_type(info: &str) -> Self {
        let desc = format!("bad sale_type: '{}'", info);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn no_contract(contract_id: &ContractId) -> Self {
        let desc = format!("contract wasn't found '{}'", contract_id);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn bad_contract_state(status: ContractStatus, action: &str) -> Self {
        let desc = format!("bad contract state '{:?}' for action '{}'", status, action);
        Error::with_info(Code::BadState, desc)
    }

    pub fn bad_conditions(conditions: &Conditions) -> Self {
        let desc = format!("bad conditions: {:?}", conditions);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn mismatched_doc_list() -> Self {
        let desc = format!("document lists do not match");
        Error::with_info(Code::BadValue, desc)
    }

    pub fn bad_content_type() -> Self {
        let desc = format!("Content type error. Check parameters sent");
        Error::with_info(Code::BadParam, desc)
    }

    pub fn unable_to_send_msg(info: &str) -> Self {
        let desc = format!("unable to send message: {}", info);
        Error::with_info(Code::Other, desc)
    }

    pub fn bad_stored_member(info: &str) -> Self {
        let desc = format!(
            "Unable to convert stored MemberIdentity to PublicKey: {}",
            info
        );
        Error::with_info(Code::Other, desc)
    }

    pub fn bad_file_type(file_type: &str) -> Self {
        let desc = format!("Bad file type: {:?}", file_type);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn unexpected_file_type() -> Self {
        let desc = format!("Unexpected file type");
        Error::with_info(Code::Unexpected, desc)
    }

    pub fn deed_file_not_found(contract_id: &ContractId) -> Self {
        let desc = format!("Deed file for contract '{}' wasn't found", contract_id);
        Error::with_info(Code::NotFound, desc)
    }

    pub fn application_file_not_found(contract_id: &ContractId) -> Self {
        let desc = format!(
            "Application file for contract '{}' wasn't found",
            contract_id
        );
        Error::with_info(Code::NotFound, desc)
    }

    pub fn mismatched_deed_files() -> Self {
        let desc = "Deed files do not match";
        Error::with_info(Code::BadValue, desc.to_owned())
    }

    pub fn mismatched_application_files() -> Self {
        let desc = "Application files do not match";
        Error::with_info(Code::BadValue, desc.to_owned())
    }

    pub fn check_failed(errors: Vec<Check>) -> Self {
        let desc = format!("Following checks failed: {:?}", errors);
        Error::with_info(Code::CheckFail, desc)
    }

    pub fn internal_bad_struct(structure: &str) -> Self {
        warn!("Stored structure has bad format: {}", structure);
        let desc = "Stored structure has bad format";
        Error::with_info(Code::Internal, desc.to_owned())
    }

    pub fn no_reference_number(contract_id: &ContractId) -> Self {
        let desc = format!(
            "reference number for contract '{}' from fips wasn't found ",
            contract_id
        );
        Error::with_info(Code::NotFound, desc)
    }

    pub fn invalid_string_length(expected: usize) -> Self {
        let desc = format!("expecting string with length = {}", expected);
        Error::with_info(Code::BadValue, desc)
    }

    pub fn ok<T>(self) -> Result<T> {
        Err(self)
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Error", 3)?;
        state.serialize_field("name", &self.code)?;
        state.serialize_field("code", &self.code())?;
        state.serialize_field("description", &self.desc)?;
        state.end()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code as u8, self.desc.as_str())
    }
}

impl Error {
    fn with_info(code: Code, desc: String) -> Self {
        Error { code, desc }
    }

    pub fn code(&self) -> u8 {
        self.code as u8
    }

    pub fn info(&self) -> &str {
        self.desc.as_str()
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(err: std::convert::Infallible) -> Self {
        panic!("infallible error occured: {}", err)
    }
}

impl From<blockp_core::crypto::CryptoError> for Error {
    fn from(err: blockp_core::crypto::CryptoError) -> Self {
        let desc = format!("{}", err);
        Error::with_info(Code::Crypto, desc)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        let desc = format!("{}", err);
        let splitter: Vec<String> = desc.splitn(2, ": ").map(|s| String::from(s)).collect();

        // if we have our error desc inside of JSON error desc
        if splitter.len() == 2 {
            let code = {
                let code = if let Some(n) = splitter.get(0) {
                    n.parse::<u8>().ok()
                } else {
                    None
                };

                let desc = splitter.get(1);
                let code = match (code, desc) {
                    (Some(n), Some(_d)) => Code::try_from(n).ok(),
                    _ => None,
                };

                match (code, desc) {
                    (Some(c), Some(d)) => Some((c, d.clone())),
                    _ => None,
                }
            };
            match code {
                Some(c) => return Error::with_info(c.0, c.1),
                _ => (),
            }
        }

        let desc = format!("bad JSON '{}'", err);
        Error::with_info(Code::BadValue, desc)
    }
}

impl From<hex::FromHexError> for Error {
    fn from(err: hex::FromHexError) -> Self {
        let desc = format!("bad HEX '{}'", err);
        Error::with_info(Code::BadValue, desc)
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        let desc = format!("bad UTF8 '{}'", err);
        Error::with_info(Code::BadValue, desc)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        let desc = format!("bad UTF8 '{}'", err);
        Error::with_info(Code::BadValue, desc)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Self {
        let desc = format!("bad BASE64 '{}'", err);
        Error::with_info(Code::BadValue, desc)
    }
}

impl From<std::str::ParseBoolError> for Error {
    fn from(err: std::str::ParseBoolError) -> Self {
        let desc = format!("parse bool error: {}", err);
        Error::with_info(Code::BadParam, desc)
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(err: std::num::ParseFloatError) -> Self {
        let desc = format!("parse float error: {}", err);
        Error::with_info(Code::BadParam, desc)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        let desc = format!("parse integer error: {}", err);
        Error::with_info(Code::BadParam, desc)
    }
}

impl From<Code> for Error {
    fn from(code: Code) -> Self {
        Error::with_info(code, code.to_string())
    }
}

impl From<actix_web::error::PayloadError> for Error {
    fn from(err: actix_web::error::PayloadError) -> Self {
        let desc = format!("bad payload: {}", err);
        Error::with_info(Code::BadParam, desc)
    }
}

impl From<actix_web::error::JsonPayloadError> for Error {
    fn from(err: actix_web::error::JsonPayloadError) -> Self {
        match err {
            actix_web::error::JsonPayloadError::Overflow => {
                let desc = "bad payload: Payload size is bigger than allowed. (default: 256kB)";
                Error::with_info(Code::BadParam, desc.to_owned())
            }
            actix_web::error::JsonPayloadError::ContentType => Error::bad_content_type(),
            actix_web::error::JsonPayloadError::Deserialize(err) => err.into(),
            actix_web::error::JsonPayloadError::Payload(err) => err.into(),
        }
    }
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::BadRequest().json(response::Errors::from(self.clone()))
    }
}

pub trait FutureResponseError {
    fn error_future_response(self) -> FutureResponse;
}

impl FutureResponseError for Error {
    fn error_future_response(self) -> FutureResponse {
        let response = self.error_response();
        let f = Ok(response).into_future();
        let bf: FutureResponse = Box::new(f);
        bf
    }
}

pub type Result<T> = std::result::Result<T, Error>;
