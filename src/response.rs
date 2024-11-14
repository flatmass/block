use std::collections::HashMap;

use actix_web::{AsyncResponder, HttpResponse};
use futures::future::Future;

use crate::api::ObjectsCounter;
use crate::data::attachment::Attachment;
use crate::data::conditions::CheckKey;
use crate::data::contract::ContractStatus;
use crate::data::cost::Cost;
use crate::data::object::ObjectIdentity;
use crate::dto::{
    CheckInfo, ConditionsInfo, ContractInfo, LotInfoWithObjects, Lots, ObjectData, ObjectInfo,
    TxHash, TxList,
};
use crate::error::{Error, Result};

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Success,
    Failure,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Data {
    Empty,
    TxHash(TxHash),
    Objects(Vec<ObjectIdentity>),
    #[serde(rename(serialize = "lot"))]
    LotInfoWithObjects(LotInfoWithObjects),
    Bids(Vec<Cost>),
    TxHashes(Vec<String>),
    Attachment(Attachment),
    ObjectInfo(ObjectInfo),
    Checks(HashMap<CheckKey, CheckInfo>),
    #[serde(rename(serialize = "status"))]
    ContractStatus(String),
    ContractInfo(ContractInfo),
    #[serde(rename(serialize = "conditions"))]
    ConditionsInfo(ConditionsInfo),
    #[serde(rename(serialize = "object_info"))]
    ObjectData(ObjectData),
    #[serde(rename(serialize = "objects_counter"))]
    ObjectsCounter(usize),
}

impl Data {
    pub fn is_empty(&self) -> bool {
        self.eq(&Data::Empty)
    }
}

impl From<TxHash> for Data {
    fn from(tx_hash: TxHash) -> Self {
        Self::TxHash(tx_hash)
    }
}

impl From<ObjectInfo> for Data {
    fn from(object_info: ObjectInfo) -> Self {
        Self::ObjectInfo(object_info)
    }
}

impl From<Vec<ObjectIdentity>> for Data {
    fn from(object_vec: Vec<ObjectIdentity>) -> Self {
        Self::Objects(object_vec)
    }
}

impl From<LotInfoWithObjects> for Data {
    fn from(lot_info: LotInfoWithObjects) -> Self {
        Self::LotInfoWithObjects(lot_info)
    }
}

impl From<Lots> for Data {
    fn from(lots: Lots) -> Self {
        let lots = lots.iter().map(|lot_id| lot_id.to_hex()).collect();
        Self::TxHashes(lots)
    }
}

impl From<Vec<Cost>> for Data {
    fn from(bids: Vec<Cost>) -> Self {
        Self::Bids(bids)
    }
}

impl From<TxList> for Data {
    fn from(tx_list: TxList) -> Self {
        Self::TxHashes(tx_list.0)
    }
}

impl From<Attachment> for Data {
    fn from(attachment: Attachment) -> Self {
        Self::Attachment(attachment)
        /*Self::Attachment {
            name: attachment.name().to_owned(),
            data: base64::encode(attachment.data())
        }*/
    }
}

impl From<HashMap<CheckKey, CheckInfo>> for Data {
    fn from(checks: HashMap<CheckKey, CheckInfo>) -> Self {
        Self::Checks(checks)
    }
}

impl From<ContractStatus> for Data {
    fn from(status: ContractStatus) -> Self {
        Self::ContractStatus(status.to_string())
    }
}

impl From<ContractInfo> for Data {
    fn from(contract: ContractInfo) -> Self {
        Self::ContractInfo(contract)
    }
}

impl From<ConditionsInfo> for Data {
    fn from(conditions: ConditionsInfo) -> Self {
        Self::ConditionsInfo(conditions)
    }
}

impl From<ObjectData> for Data {
    fn from(object_data: ObjectData) -> Self {
        Self::ObjectData(object_data)
    }
}

impl From<ObjectsCounter> for Data {
    fn from(counter: ObjectsCounter) -> Self {
        Self::ObjectsCounter(counter.objects)
    }
}

#[derive(Debug, Serialize)]
struct ApiResult {
    status: Status,
    #[serde(skip_serializing_if = "Data::is_empty")]
    data: Data,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<Error>,
}

impl ApiResult {
    pub fn success() -> Self {
        ApiResult {
            status: Status::Success,
            data: Data::Empty,
            errors: Vec::new(),
        }
    }

    pub fn failure() -> Self {
        ApiResult {
            status: Status::Failure,
            data: Data::Empty,
            errors: Vec::new(),
        }
    }

    pub fn with(mut self, data: Data) -> Self {
        self.data = data;
        self
    }

    pub fn err(mut self, error: Error) -> Self {
        self.errors.push(error);
        self
    }
}

#[derive(Debug, Serialize)]
pub struct Errors {
    errors: Vec<Error>,
}

impl From<Vec<Error>> for Errors {
    fn from(errors: Vec<Error>) -> Errors {
        Errors { errors }
    }
}

impl From<Error> for Errors {
    fn from(err: Error) -> Errors {
        Errors { errors: vec![err] }
    }
}

pub type Response = std::result::Result<actix_web::HttpResponse, actix_web::Error>;

pub trait ToWeb {
    fn to_web(self) -> Response;
}

impl ToWeb for ApiResult {
    fn to_web(self) -> Response {
        match self.status {
            Status::Success => Ok(actix_web::HttpResponse::Ok().json(self)),
            Status::Failure => {
                Ok(actix_web::HttpResponse::BadRequest().json(Errors::from(self.errors)))
            }
        }
    }
}

impl ToWeb for () {
    fn to_web(self) -> Response {
        ApiResult::success().to_web()
    }
}

impl<T: Into<Data>> ToWeb for Result<T> {
    fn to_web(self) -> Response {
        match self {
            Ok(data) => ApiResult::success().with(data.into()).to_web(),
            Err(e) => ApiResult::failure().err(e).to_web(),
        }
    }
}

impl<T: Into<Data>> ToWeb for T {
    fn to_web(self) -> Response {
        ApiResult::success().with(self.into()).to_web()
    }
}

impl ToWeb for Error {
    fn to_web(self) -> Response {
        ApiResult::failure().err(self).to_web()
    }
}

pub trait IntoResponse<I>: Sized {
    fn into_response(self) -> actix_web::FutureResponse<HttpResponse, actix_web::Error>;
}

impl<F, I> IntoResponse<I> for F
where
    F: Future<Item = I, Error = Error> + 'static,
    I: ToWeb + 'static,
{
    fn into_response(self) -> actix_web::FutureResponse<HttpResponse, actix_web::Error> {
        self.then(|r| match r {
            Ok(data) => data.to_web(),
            Err(err) => err.to_web(),
        })
        .responder()
    }
}
