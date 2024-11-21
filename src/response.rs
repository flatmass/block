use std::collections::HashMap;

use actix_web::{AsyncResponder, HttpResponse};
use futures::future::Future;
use serde::Serialize;

#[cfg(feature = "extra_counter")]
use crate::api::{ContractsCounter, LotsCounter, ObjectsCounter};
use blockp_core::crypto::Hash;

use crate::data::conditions::CheckKey;
use crate::data::contract::ContractStatus;
use crate::data::cost::Cost;
use crate::dto::{
    AttachmentDto, CheckInfo, ConditionsInfo, ContractInfo, HashWrapperDto, LotInfoWithObjects,
    ObjectIdentityDto, ObjectInformationDto, ObjectParticipates, PaginationPage, RequestConfirmDto,
    TxHash, TxList,
};
use crate::error::{Error, Result};

#[cfg(feature = "internal_api")]
use crate::dto::MemberEsiaTokenDto;

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
    Objects(Vec<ObjectIdentityDto>),
    #[serde(rename(serialize = "lot"))]
    LotInfoWithObjects(LotInfoWithObjects),
    Bids(Vec<Cost>),
    TxHashes(Vec<String>),
    Attachment(AttachmentDto),
    Checks(HashMap<CheckKey, CheckInfo>),
    #[serde(rename(serialize = "status"))]
    ContractStatus(String),
    ContractInfo(ContractInfo),
    #[serde(rename(serialize = "conditions"))]
    ConditionsInfo(ConditionsInfo),
    #[serde(rename(serialize = "object"))]
    ObjectData(ObjectInformationDto),
    #[serde(rename(serialize = "participates"))]
    ObjectParticipates(ObjectParticipates),
    #[serde(rename(serialize = "page"))]
    PageLots(PaginationPage<HashWrapperDto<LotInfoWithObjects>, Option<Hash>>),
    #[serde(rename(serialize = "page"))]
    PageObjects(PaginationPage<ObjectInformationDto, Option<ObjectIdentityDto>>),
    #[cfg(feature = "internal_api")]
    Token(MemberEsiaTokenDto),
    Status(RequestConfirmDto),
    #[cfg(feature = "extra_counter")]
    #[serde(rename(serialize = "objects_counter"))]
    ObjectsCounter(usize),
    #[cfg(feature = "extra_counter")]
    #[serde(rename(serialize = "lots"))]
    LotsCounter(LotsCounter),
    #[cfg(feature = "extra_counter")]
    #[serde(rename(serialize = "contracts"))]
    ContractsCounter(ContractsCounter),
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

impl From<Vec<ObjectIdentityDto>> for Data {
    fn from(object_vec: Vec<ObjectIdentityDto>) -> Self {
        Self::Objects(object_vec)
    }
}

impl From<LotInfoWithObjects> for Data {
    fn from(lot_info: LotInfoWithObjects) -> Self {
        Self::LotInfoWithObjects(lot_info)
    }
}

impl From<Vec<Hash>> for Data {
    fn from(hashes: Vec<Hash>) -> Self {
        let hashes = hashes.iter().map(|hash| hash.to_hex()).collect();
        Self::TxHashes(hashes)
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

impl From<ObjectInformationDto> for Data {
    fn from(object_data: ObjectInformationDto) -> Self {
        Self::ObjectData(object_data)
    }
}

impl From<ObjectParticipates> for Data {
    fn from(data: ObjectParticipates) -> Self {
        Self::ObjectParticipates(data)
    }
}

impl From<PaginationPage<HashWrapperDto<LotInfoWithObjects>, Option<Hash>>> for Data {
    fn from(data: PaginationPage<HashWrapperDto<LotInfoWithObjects>, Option<Hash>>) -> Self {
        Self::PageLots(data)
    }
}

impl From<PaginationPage<ObjectInformationDto, Option<ObjectIdentityDto>>> for Data {
    fn from(data: PaginationPage<ObjectInformationDto, Option<ObjectIdentityDto>>) -> Self {
        Self::PageObjects(data)
    }
}

impl From<RequestConfirmDto> for Data {
    fn from(data: RequestConfirmDto) -> Self {
        Self::Status(data)
    }
}

impl From<AttachmentDto> for Data {
    fn from(data: AttachmentDto) -> Self {
        Self::Attachment(data)
    }
}

#[cfg(feature = "extra_counter")]
impl From<ObjectsCounter> for Data {
    fn from(counter: ObjectsCounter) -> Self {
        Self::ObjectsCounter(counter.objects)
    }
}

#[cfg(feature = "extra_counter")]
impl From<LotsCounter> for Data {
    fn from(counter: LotsCounter) -> Self {
        Self::LotsCounter(counter)
    }
}

#[cfg(feature = "extra_counter")]
impl From<ContractsCounter> for Data {
    fn from(counter: ContractsCounter) -> Self {
        Self::ContractsCounter(counter)
    }
}

#[cfg(feature = "internal_api")]
impl From<MemberEsiaTokenDto> for Data {
    fn from(token: MemberEsiaTokenDto) -> Self {
        Self::Token(token)
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

impl ToWeb for &'static [u8] {
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

#[cfg(test)]
pub(crate) mod test {
    use std::str::FromStr;

    use blockp_core::crypto::Hash;

    use crate::data::object::ObjectIdentity;
    use crate::dto::ObjectIdentityDto;

    use super::*;

    #[test]
    fn se_data_objects() {
        let data_raw = vec![
            ObjectIdentity::from_str("trademark::111").unwrap(),
            ObjectIdentity::from_str("trademark::222").unwrap(),
            ObjectIdentity::from_str("trademark::333").unwrap(),
        ];

        let data: Vec<ObjectIdentityDto> = data_raw.into_iter().map(|v| v.into()).collect();
        let value: Data = data.into();

        let expected = r#"{"objects":[{"class":1,"reg_number":"111"},{"class":1,"reg_number":"222"},{"class":1,"reg_number":"333"}]}"#;

        assert_eq!(expected, serde_json::to_string(&value).unwrap());
    }

    #[test]
    fn se_object_participates() {
        let lots = vec![Hash::from_str(
            "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
        )
        .unwrap()];

        let contracts = vec![Hash::from_str(
            "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
        )
        .unwrap()];

        let data: ObjectParticipates = ObjectParticipates { lots, contracts };
        let value: Data = data.into();

        let expected = r#"{"participates":{"lots":["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"],"contracts":["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"]}}"#;

        assert_eq!(expected, serde_json::to_string(&value).unwrap());
    }
}
