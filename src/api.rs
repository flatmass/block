use std::collections::HashMap;
#[cfg(feature = "internal_api")]
use std::collections::HashSet;
#[cfg(feature = "extra_counter")]
use std::convert::TryFrom;
#[cfg(feature = "extra_counter")]
use std::ops::AddAssign;
use std::sync::Arc;

use actix_web::http::Method;
use actix_web::HttpMessage;
use chrono::{DateTime, Utc};
use futures::{Future, IntoFuture, Stream};
use serde::Deserialize;
#[cfg(feature = "extra_counter")]
use serde::Serialize;

use blockp_core::api::backends::actix::{FutureResponse, HttpRequest, ResourceHandler};
use blockp_core::api::{ServiceApiBackend, ServiceApiBuilder};
use blockp_core::crypto::Hash;

use crate::control;
use crate::data::conditions::CheckKey;
use crate::data::contract::ContractId;
#[cfg(feature = "internal_api")]
use crate::data::contract::ContractStatus;
use crate::data::cost::Cost;
use crate::data::lot::LotStatus;
#[cfg(feature = "internal_api")]
use crate::data::member::MemberIdentity;
#[cfg(feature = "internal_api")]
use crate::data::object::ObjectIdentity;
use crate::data::payment::PaymentStatus;
#[cfg(feature = "internal_api")]
use crate::data::payment::{Calculation, PaymentDetail};
#[cfg(feature = "internal_api")]
use crate::data::strings::verify_node_name;
use crate::dto::{
    CalculationInfo, CheckInfo, ConditionsInfo, HashInfo, LotInfo, MemberInfo, ObjectIdentityDto,
    PaymentDetailsInfo, SignInfo,
};
use crate::error::{Error, FutureResponseError};
use crate::response::IntoResponse;
#[cfg(feature = "extra_counter")]
use crate::schema::Schema;
use crate::upload::handle_multipart_item;
use crate::util::get_from_multipart_map;
use crate::util::{get_attachment_from_map, get_str_from_map};
#[cfg(feature = "internal_api")]
use crate::util::{get_attachment_nullable_from_map, get_string_from_map};
use crate::util::{get_from_map, get_from_map_nullable};

pub struct OwnershipApi;

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct JustObjectIdentity {
    object: ObjectIdentityDto,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct JustContractTxHash {
    contract_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct GetObjectHistory {
    object: ObjectIdentityDto,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct AddParticipant {
    member: MemberInfo,
    node: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct AcquireLot {
    requestor: MemberInfo,
    lot_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct OpenLot {
    requestor: MemberInfo,
    lot: LotInfo,
    ownership: ConditionsInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct CloseLot {
    requestor: MemberInfo,
    lot_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct GetLots {
    lot_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct EditLotStatus {
    lot_tx_hash: HashInfo,
    status: LotStatus,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ExtendLotPeriod {
    requestor: MemberInfo,
    lot_tx_hash: HashInfo,
    new_expiration: DateTime<Utc>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct PurchaseOffer {
    requestor: MemberInfo,
    buyer: MemberInfo,
    rightholder: MemberInfo,
    price: Cost,
    conditions: ConditionsInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct AddBid {
    lot_tx_hash: HashInfo,
    value: Cost,
    requestor: MemberInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct PublishBids {
    bids: Vec<Cost>,
    lot_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ApproveContract {
    contract_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct DeleteContractFiles {
    requestor: MemberInfo,
    contract_tx_hash: HashInfo,
    doc_tx_hashes: Vec<HashInfo>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ConfirmContract {
    requestor: MemberInfo,
    contract_tx_hash: HashInfo,
    deed_tx_hash: HashInfo,
    application_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct RefuseContract {
    requestor: MemberInfo,
    contract_tx_hash: HashInfo,
    reason: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct DraftContract {
    contract_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct RejectContract {
    contract_tx_hash: HashInfo,
    reason: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct UpdateContract {
    contract_tx_hash: HashInfo,
    requestor: MemberInfo,
    price: Cost,
    conditions: ConditionsInfo,
    contract_correspondence: Option<String>,
    objects_correspondence: Option<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ExecuteLot {
    lot_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct LotUndefined {
    lot_tx_hash: HashInfo,
    admit: bool,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ContractUndefined {
    contract_tx_hash: HashInfo,
    admit: bool,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct DeleteFiles {
    requestor: MemberInfo,
    doc_tx_hashes: Vec<HashInfo>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct AddAttachmentSign {
    requestor: MemberInfo,
    doc_tx_hash: HashInfo,
    sign: SignInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ContractHash {
    contract_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct SignContract {
    requestor: MemberInfo,
    contract_tx_hash: HashInfo,
    application_sign: SignInfo,
    deed_sign: SignInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ContractSubmitChecks {
    contract_tx_hash: HashInfo,
    checks: HashMap<CheckKey, CheckInfo>,
    is_undef: bool,
    reference_number: Option<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct LotSubmitChecks {
    lot_tx_hash: HashInfo,
    checks: HashMap<CheckKey, CheckInfo>,
    is_undef: bool,
    reference_number: Option<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ReferenceContractNumber {
    contract_tx_hash: HashInfo,
    reference_number: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct TaxRequest {
    requestor: MemberInfo,
    contract_tx_hash: HashInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct TaxContractCalculation {
    contract_tx_hash: HashInfo,
    calculations: Vec<CalculationInfo>,
    reference_number: Option<String>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct MemberToken {
    member: MemberInfo,
    token: String,
    oid: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ConfirmCreate {
    requestor: MemberInfo,
    contract_tx_hash: HashInfo,
}

impl TaxContractCalculation {
    #[cfg(feature = "internal_api")]
    pub fn is_valid(&self) -> Result<(), Error> {
        if self.calculations.is_empty() {
            Error::empty_param("calculations").ok()?;
        };
        if let Some(ref ref_numb) = self.reference_number {
            if ref_numb.is_empty() {
                Error::empty_param("reference_number").ok()?
            };

            if ref_numb.len() > 256 {
                Error::too_long_param("reference_number").ok()?
            };
        }
        let mut set = HashSet::with_capacity(self.calculations.len());
        for x in &self.calculations {
            x.is_valid()?;
            if !set.insert(x.id.as_str()) {
                Error::duplicate_values("calculations").ok()?;
            };
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct TaxLotCalculation {
    lot_tx_hash: HashInfo,
    calculations: Vec<CalculationInfo>,
    reference_number: Option<String>,
}

impl TaxLotCalculation {
    #[cfg(feature = "internal_api")]
    pub fn is_valid(&self) -> Result<(), Error> {
        if self.calculations.is_empty() {
            Error::empty_param("calculations").ok()?;
        };
        if let Some(ref ref_numb) = self.reference_number {
            if ref_numb.is_empty() {
                Error::empty_param("reference_number").ok()?
            };

            if ref_numb.len() > 256 {
                Error::too_long_param("reference_number").ok()?
            };
        };
        let mut set = HashSet::with_capacity(self.calculations.len());
        for x in &self.calculations {
            x.is_valid()?;
            if !set.insert(x.id.as_str()) {
                Error::duplicate_values("calculations").ok()?;
            };
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct TaxWithPaymentDetails {
    contract_tx_hash: HashInfo,
    payment_details: Vec<PaymentDetailsInfo>,
}

impl TaxWithPaymentDetails {
    #[cfg(feature = "internal_api")]
    pub fn is_valid(&self) -> crate::error::Result<()> {
        if self.payment_details.is_empty() {
            Error::empty_param("payment_details").ok()?;
        };
        let mut set = HashSet::with_capacity(self.payment_details.len());
        for x in &self.payment_details {
            x.is_valid()?;
            if !set.insert(x.calculation.id.as_str()) {
                Error::duplicate_values("payment_details").ok()?;
            };
        }
        Ok(())
    }
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct TaxStatus {
    contract_tx_hash: HashInfo,
    calculation_id: String,
    status: PaymentStatus,
}

impl TaxStatus {
    #[cfg(feature = "internal_api")]
    pub fn is_valid(&self) -> crate::error::Result<()> {
        if self.calculation_id.is_empty() {
            Error::empty_param("id").ok()?
        }
        if self.calculation_id.len() > 256 {
            Error::too_long_param("id").ok()?
        }
        Ok(())
    }
}

#[cfg(feature = "extra_counter")]
#[derive(Debug, Eq, PartialEq)]
pub struct ObjectsCounter {
    pub objects: usize,
}

#[cfg(feature = "extra_counter")]
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct LotsCounter {
    pub stats: HashMap<String, u64>,
}

#[cfg(feature = "extra_counter")]
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ContractsCounter {
    pub stats: HashMap<String, u64>,
}

impl OwnershipApi {
    #[cfg(feature = "internal_api")]
    fn add_object(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .from_err()
            .collect()
            .and_then(move |params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let object = get_from_multipart_map(&params, "object")?;
                let data = get_str_from_map(&params, "data")?;
                let ownership_str = get_str_from_map(&params, "ownership")?;
                let ownership = serde_json::from_str(ownership_str)
                    .map_err(|e| Error::bad_json(ownership_str, e))?;
                control::add_object(state, object, data, ownership)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn update_object(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .from_err()
            .collect()
            .and_then(move |params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let object = get_from_multipart_map(&params, "object")?;
                let data = get_str_from_map(&params, "data")?;
                let ownership_str = get_str_from_map(&params, "ownership")?;
                let ownership = serde_json::from_str(ownership_str)
                    .map_err(|e| Error::bad_json(ownership_str, e))?;
                control::update_object(state, object, data, ownership)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn get_object_participates(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_string_from_map(&query, "reg_number")
            .into_future()
            .join(get_from_map(&query.clone(), "class"))
            .and_then(|(reg_number, class)| {
                let object_id = ObjectIdentity::new(class, reg_number.as_str());
                if !object_id.is_valid() {
                    Error::bad_object_format(&object_id.to_string(), "invalid number").ok()?
                }
                control::object_participates(state, object_id)
            })
            .into_response()
    }

    fn get_object_history(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(move |GetObjectHistory { object }| {
                control::object_history(state, object.into())
            })
            .into_response()
    }

    fn get_bids(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "lot_tx_hash")
            .into_future()
            .and_then(|lot_tx_hash| control::get_bids(state, lot_tx_hash))
            .into_response()
    }

    fn add_bid(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: AddBid| {
                control::add_bid(
                    state,
                    json.requestor.into(),
                    &json.lot_tx_hash,
                    json.value.into(),
                )
            })
            .into_response()
    }

    fn get_bid_transactions(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "lot_tx_hash")
            .into_future()
            .and_then(|lot_tx_hash| control::get_bid_transactions(state, lot_tx_hash))
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn publish_bids(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: PublishBids| {
                if json.bids.is_empty() {
                    Error::empty_param("bids").ok()?
                }

                control::publish_bids(
                    state,
                    &json.lot_tx_hash,
                    json.bids
                        .into_iter()
                        .map(|cost_info| cost_info.into())
                        .collect(),
                )
            })
            .into_response()
    }

    fn open_lot(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: OpenLot| {
                control::open_lot(state, params.requestor.into(), params.lot, params.ownership)
            })
            .into_response()
    }

    fn close_lot(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: CloseLot| {
                control::close_lot(state, params.requestor.into(), &params.lot_tx_hash)
            })
            .into_response()
    }

    fn get_lots(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        if query.contains_key("member") {
            get_from_map(&query, "member")
                .into_future()
                .and_then(|member| control::get_member_lots(state, member))
                .into_response()
        } else if query.contains_key("lot_tx_hash") {
            get_from_map(&query, "lot_tx_hash")
                .into_future()
                .and_then(|lot_tx_hash: HashInfo| {
                    control::get_lot_info_with_objects(state, &lot_tx_hash)
                })
                .into_response()
        } else if query.contains_key("limit") {
            get_from_map(&query, "limit")
                .into_future()
                .join(get_from_map_nullable(&query, "from"))
                .and_then(|(limit, from)| Ok(control::get_lots_pagination(state, limit, from)))
                .into_response()
        } else {
            control::get_all_lots(state).into_future().into_response()
        }
    }

    fn extend_lot_period(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ExtendLotPeriod| {
                control::extend_lot_period(
                    state,
                    params.requestor.into(),
                    &params.lot_tx_hash,
                    params.new_expiration,
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn edit_lot_status(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: EditLotStatus| {
                if params.status != LotStatus::Rejected
                    && params.status != LotStatus::Verified
                    && params.status != LotStatus::Closed
                {
                    Error::bad_state("lot status should be 'rejected', 'verified', 'closed'")
                        .ok()?
                };
                control::edit_lot_status(state, &params.lot_tx_hash, params.status)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn execute_lot(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ExecuteLot| control::execute_lot(state, &params.lot_tx_hash))
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn undefined(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: LotUndefined| {
                control::lot_undefined(state, &params.lot_tx_hash, params.admit)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn contract_undefined(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ContractUndefined| {
                control::contract_undefined(state, &params.contract_tx_hash, params.admit)
            })
            .into_response()
    }

    fn acquire_lot(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: AcquireLot| {
                control::acquire_lot(state, params.requestor.into(), &params.lot_tx_hash)
            })
            .into_response()
    }

    fn purchase_offer(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(move |params: PurchaseOffer| {
                control::purchase_offer(
                    state,
                    params.requestor.into(),
                    params.buyer.into(),
                    params.rightholder.into(),
                    params.price.into(),
                    params.conditions.into(),
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn draft_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();

        req.json()
            .from_err()
            .and_then(|params: DraftContract| {
                control::draft_contract(state, &params.contract_tx_hash)
            })
            .into_response()
    }

    fn get_objects(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        if query.contains_key("owner") {
            get_from_map(&query, "owner")
                .into_future()
                .and_then(|owner| control::get_member_objects(state, owner))
                .into_response()
        } else if query.contains_key("object") {
            get_from_map(&query, "object")
                .into_future()
                .and_then(|object| control::get_object(state, object))
                .into_response()
        } else if query.contains_key("limit") {
            get_from_map(&query, "limit")
                .into_future()
                .join(get_from_map_nullable(&query, "from"))
                .and_then(|(limit, offset)| {
                    Ok(control::get_objects_pagination(state, limit, offset))
                })
                .into_response()
        } else {
            Error::empty_param("object").error_future_response()
        }
    }

    // fn attach_file(req: HttpRequest) -> FutureResponse {
    //     let state = req.state().clone();
    //     req.multipart()
    //         .map(handle_multipart_item)
    //         .flatten()
    //         .from_err()
    //         .collect()
    //         .and_then(|params: Vec<(String, Vec<u8>)>| {
    //             let mut members = vec![];
    //             for (key, value) in params.iter() {
    //                 if key == "members" {
    //                     if value.is_empty() {
    //                         Error::empty_param("members").ok()?
    //                     }
    //                     let value = String::from_utf8(value.clone()).map_err(Error::from)?;
    //                     let member = MemberIdentity::from_str(value.as_str())?;
    //                     members.push(member);
    //                 }
    //             }
    //             let params = params.into_iter().collect();
    //             let requestor = get_from_multipart_map(&params, "requestor")?;
    //             let data = get_slice_from_map(&params, "file")?;
    //             let name = get_str_from_map(&params, "name")?;
    //             let file_type = get_from_multipart_map(&params, "file_type")?;
    //             verify_filename(name)?;
    //             control::attach_file(state, requestor, name, data, file_type, members)
    //         })
    //         .into_response()
    // }

    // fn delete_files(req: HttpRequest) -> FutureResponse {
    //     let state = req.state().clone();
    //     req.json()
    //         .from_err()
    //         .and_then(
    //             move |DeleteFiles {
    //                       requestor,
    //                       doc_tx_hashes,
    //                   }| {
    //                 control::delete_files(
    //                     state,
    //                     requestor.into(),
    //                     &doc_tx_hashes
    //                         .into_iter()
    //                         .map(|v| v.into())
    //                         .collect::<Vec<Hash>>(),
    //                 )
    //             },
    //         )
    //         .into_response()
    // }

    // fn add_attachment_sign(req: HttpRequest) -> FutureResponse {
    //     let state = req.state().clone();
    //     req.json()
    //         .from_err()
    //         .and_then(
    //             move |AddAttachmentSign {
    //                       requestor,
    //                       doc_tx_hash,
    //                       sign,
    //                   }| {
    //                 control::add_attachment_sign(state, requestor.into(), &doc_tx_hash, sign.into())
    //             },
    //         )
    //         .into_response()
    // }

    fn get_file(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        let requestor = get_from_map(&query, "requestor");

        get_from_map(&query, "doc_tx_hash")
            .into_future()
            .and_then(|doc_tx_hash: HashInfo| {
                control::get_file(state, Some(&requestor?), &doc_tx_hash)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn get_file_private(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();

        get_from_map(&query, "doc_tx_hash")
            .into_future()
            .and_then(|doc_tx_hash: HashInfo| control::get_file(state, None, &doc_tx_hash))
            .into_response()
    }

    fn attach_contract_other_file(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .from_err()
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let requestor = get_from_multipart_map(&params, "requestor")?;
                let contract_tx_hash = get_str_from_map(&params, "contract_tx_hash")?.parse()?;
                let attachment = get_attachment_from_map(&params)?;
                control::attach_contract_other_file(state, requestor, &contract_tx_hash, attachment)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn attach_contract_main_file(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .from_err()
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let contract_tx_hash = get_str_from_map(&params, "contract_tx_hash")?.parse()?;
                let attachment = get_attachment_from_map(&params)?;
                control::attach_contract_main_file(state, &contract_tx_hash, attachment)
            })
            .into_response()
    }

    fn delete_contract_files(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();

        req.json()
            .from_err()
            .and_then(
                |DeleteContractFiles {
                     requestor,
                     contract_tx_hash,
                     doc_tx_hashes,
                 }| {
                    if doc_tx_hashes.is_empty() {
                        Error::empty_param("doc_tx_hashes").ok()?
                    }
                    control::delete_contract_files(
                        state,
                        requestor.into(),
                        &contract_tx_hash,
                        &doc_tx_hashes
                            .into_iter()
                            .map(|v| v.into())
                            .collect::<Vec<Hash>>(),
                    )
                },
            )
            .into_response()
    }

    fn confirm_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();

        req.json()
            .from_err()
            .and_then(|params: ConfirmContract| {
                control::confirm_contract(
                    state,
                    params.requestor.into(),
                    &params.contract_tx_hash,
                    &params.deed_tx_hash,
                    &params.application_tx_hash,
                )
            })
            .into_response()
    }

    fn refuse_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(
                |RefuseContract {
                     requestor,
                     contract_tx_hash,
                     reason,
                 }| {
                    if reason.is_empty() {
                        Error::empty_param("reason").ok()?
                    }
                    control::refuse_contract(state, requestor.into(), &contract_tx_hash, &reason)
                },
            )
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn approve_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .from_err()
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let contract_tx_hash = get_str_from_map(&params, "contract_tx_hash")?.parse()?;
                let attachment = get_attachment_nullable_from_map(&params)?;
                control::approve_contract(state, &contract_tx_hash, attachment)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn reject_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .from_err()
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let contract_tx_hash = get_str_from_map(&params, "contract_tx_hash")?.parse()?;
                let reason = get_str_from_map(&params, "reason")?;
                let attachment = get_attachment_nullable_from_map(&params)?;
                control::reject_contract(state, &contract_tx_hash, reason, attachment)
            })
            .into_response()
    }

    fn get_contract_checks(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        let requestor = get_from_map(&query, "requestor");
        get_from_map(&query, "contract_tx_hash")
            .into_future()
            .and_then(|tx_hash| control::get_contract_checks(state, requestor?, &tx_hash))
            .into_response()
    }

    fn get_lot_checks(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "lot_tx_hash")
            .into_future()
            .and_then(|tx_hash| control::get_lot_checks(state, &tx_hash))
            .into_response()
    }

    fn get_contract_status(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "contract_tx_hash")
            .into_future()
            .and_then(|tx_hash| control::get_contract_status(state, &tx_hash))
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn add_participant(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(move |AddParticipant { member, node }| {
                verify_node_name(&node)?;
                control::add_participant(state, member.into(), &node)
            })
            .into_response()
    }

    fn update_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: UpdateContract| {
                control::update_contract(
                    state,
                    &params.contract_tx_hash,
                    params.requestor.into(),
                    params.price.into(),
                    params.conditions.into(),
                    params.contract_correspondence,
                    params.objects_correspondence,
                )
            })
            .into_response()
    }

    fn get_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        if query.contains_key("member") {
            get_from_map(&query, "member")
                .into_future()
                .and_then(|member| control::get_member_contracts(state, member))
                .into_response()
        } else if query.contains_key("contract_tx_hash") {
            get_from_map(&query, "contract_tx_hash")
                .into_future()
                .and_then(|contract_tx_hash: ContractId| {
                    control::get_contract(state, &contract_tx_hash)
                })
                .into_response()
        } else {
            Error::empty_param("member").error_future_response()
        }
    }

    #[cfg(feature = "internal_api")]
    fn register_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ContractHash| {
                control::register_contract(state, &params.contract_tx_hash)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn await_user_action_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ContractHash| {
                control::await_user_action_contract(state, &params.contract_tx_hash)
            })
            .into_response()
    }

    fn sign_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: SignContract| {
                control::sign_contract(
                    state,
                    params.requestor.into(),
                    &params.contract_tx_hash,
                    params.deed_sign.into(),
                    params.application_sign.into(),
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn contract_submit_checks(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ContractSubmitChecks| {
                if let Some(ref ref_numb) = params.reference_number {
                    if ref_numb.is_empty() {
                        Error::empty_param("reference_number").ok()?
                    };

                    if ref_numb.len() > 256 {
                        Error::too_long_param("reference_number").ok()?
                    };
                }

                control::contract_submit_checks(
                    state,
                    &params.contract_tx_hash,
                    params.checks,
                    params.is_undef,
                    params.reference_number,
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn lot_submit_checks(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: LotSubmitChecks| {
                if let Some(ref ref_numb) = params.reference_number {
                    if ref_numb.is_empty() {
                        Error::empty_param("reference_number").ok()?
                    };

                    if ref_numb.len() > 256 {
                        Error::too_long_param("reference_number").ok()?
                    };
                }
                control::lot_submit_checks(
                    state,
                    &params.lot_tx_hash,
                    params.checks,
                    params.is_undef,
                    params.reference_number,
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn contract_reference_number(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ReferenceContractNumber| {
                if params.reference_number.is_empty() {
                    Error::empty_param("reference_number").ok()?
                }

                if params.reference_number.len() > 256 {
                    Error::too_long_param("reference_number").ok()?
                }

                control::contract_reference_number(
                    state,
                    &params.contract_tx_hash,
                    &params.reference_number,
                )
            })
            .into_response()
    }

    fn get_contract_conditions(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "contract_tx_hash")
            .into_future()
            .and_then(|tx_hash| control::get_contract_conditions(state, &tx_hash))
            .into_response()
    }

    fn tax_request(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: TaxRequest| {
                control::tax_request(state, json.requestor.into(), &json.contract_tx_hash)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn add_tax_contract_calculation(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: TaxContractCalculation| {
                json.is_valid()?;

                control::add_tax_contract_calculation(
                    state,
                    &json.contract_tx_hash,
                    json.calculations
                        .into_iter()
                        .map(|v| v.into())
                        .collect::<Vec<Calculation>>(),
                    json.reference_number,
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn add_tax_lot_calculation(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: TaxLotCalculation| {
                json.is_valid()?;

                control::add_tax_lot_calculation(
                    state,
                    &json.lot_tx_hash,
                    json.calculations
                        .into_iter()
                        .map(|v| v.into())
                        .collect::<Vec<Calculation>>(),
                    json.reference_number,
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn add_tax_with_payment_details(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: TaxWithPaymentDetails| {
                json.is_valid()?;

                control::add_tax_with_payment_details(
                    state,
                    &json.contract_tx_hash,
                    json.payment_details
                        .into_iter()
                        .map(|v| v.into())
                        .collect::<Vec<PaymentDetail>>(),
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn add_tax_status(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: TaxStatus| {
                json.is_valid()?;

                control::add_tax_status(
                    state,
                    &json.contract_tx_hash,
                    &json.calculation_id,
                    json.status,
                )
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn get_member_token(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "class")
            .into_future()
            .join(get_string_from_map(&query, "number"))
            .and_then(|(class, number)| {
                control::get_member_token(state, &MemberIdentity::new(class, number.as_str()))
            })
            .into_response()
    }

    fn get_confirm_create_status(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "contract_tx_hash")
            .into_future()
            .and_then(|contract_id| control::get_confirm_create_status(state, &contract_id))
            .into_response()
    }

    fn post_confirm_create(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: ConfirmCreate| {
                control::post_confirm_create(state, json.requestor.into(), &json.contract_tx_hash)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn post_unconfirm_create(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: ConfirmCreate| {
                control::post_unconfirm_create(state, json.requestor.into(), &json.contract_tx_hash)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn post_contract_new(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: JustContractTxHash| {
                control::post_contract_new(state, &json.contract_tx_hash)
            })
            .into_response()
    }

    fn put_member_token(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|json: MemberToken| {
                if json.token.is_empty() {
                    Error::empty_param("token").ok()?
                };
                if json.oid.is_empty() {
                    Error::empty_param("oid").ok()?
                };
                control::put_member_token(state, json.member.into(), &json.token, &json.oid)
            })
            .into_response()
    }

    #[cfg(feature = "extra_counter")]
    fn objects_counter(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let schema = Schema::new(state.snapshot());
        let json = ObjectsCounter {
            objects: schema.objects().keys().count().into(),
        };
        futures::future::ok::<ObjectsCounter, Error>(json).into_response()
    }

    #[cfg(feature = "extra_counter")]
    fn lots_counter(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let schema = Schema::new(state.snapshot());
        let mut counter: HashMap<String, u64> = HashMap::with_capacity(1000);
        let res: Result<(), Error> = schema.lots().keys().try_for_each(|k| {
            if let Some(value) = counter.get_mut("total") {
                value.add_assign(1);
            } else {
                counter.insert("total".to_string(), 1);
            };

            let state = schema
                .lot_states()
                .get(&k)
                .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;

            let status = LotStatus::try_from(state.status())
                .map(|status| {
                    serde_plain::to_string(&status).map_err(|_| Error::bad_state("bad lot state"))
                })
                .map_err(|_| Error::bad_lot_status(&state.status().to_string()))??;

            if let Some(value) = counter.get_mut(&status) {
                value.add_assign(1);
            } else {
                counter.insert(status, 1);
            };
            Ok(())
        });
        res.into_future()
            .and_then(|_| Ok(LotsCounter { stats: counter }))
            .into_response()
    }

    #[cfg(all(feature = "internal_api", feature = "extra_counter"))]
    fn contracts_counter(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let schema = Schema::new(state.snapshot());
        let mut counter: HashMap<String, u64> = HashMap::with_capacity(1000);
        let res: Result<(), Error> = schema.contracts().values().try_for_each(|v| {
            if let Some(value) = counter.get_mut("total") {
                value.add_assign(1);
            } else {
                counter.insert("total".to_string(), 1);
            };

            let status = ContractStatus::try_from(v.state())?.to_string();

            if let Some(value) = counter.get_mut(&status) {
                value.add_assign(1);
            } else {
                counter.insert(status, 1);
            };
            Ok(())
        });
        res.into_future()
            .and_then(|_| Ok(ContractsCounter { stats: counter }))
            .into_response()
    }

    pub fn wire(builder: &mut ServiceApiBuilder) {
        builder
            .public_scope()
            .web_backend()
            .resource(
                ResourceHandler::new("v1/lots")
                    .with(Method::GET, Arc::new(OwnershipApi::get_lots))
                    .with(Method::POST, Arc::new(OwnershipApi::open_lot))
                    .with(Method::DELETE, Arc::new(OwnershipApi::close_lot)),
            )
            .resource(
                ResourceHandler::new("v1/lots/bids")
                    .with(Method::GET, Arc::new(OwnershipApi::get_bids))
                    .with(Method::POST, Arc::new(OwnershipApi::add_bid)),
            )
            .resource(
                ResourceHandler::new("v1/lots/bids/transactions")
                    .with(Method::GET, Arc::new(OwnershipApi::get_bid_transactions)),
            )
            .resource(
                ResourceHandler::new("v1/lots/extend")
                    .with(Method::POST, Arc::new(OwnershipApi::extend_lot_period)),
            )
            .resource(
                ResourceHandler::new("v1/lots/checks")
                    .with(Method::GET, Arc::new(OwnershipApi::get_lot_checks)),
            )
            .resource(
                ResourceHandler::new("v1/contracts")
                    .with(Method::PUT, Arc::new(OwnershipApi::update_contract))
                    .with(Method::GET, Arc::new(OwnershipApi::get_contract)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/conditions")
                    .with(Method::GET, Arc::new(OwnershipApi::get_contract_conditions)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/acquire_lot")
                    .with(Method::POST, Arc::new(OwnershipApi::acquire_lot)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/purchase_offer")
                    .with(Method::POST, Arc::new(OwnershipApi::purchase_offer)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/checks")
                    .with(Method::GET, Arc::new(OwnershipApi::get_contract_checks)),
            )
            .resource(
                ResourceHandler::new("v1/tax/request")
                    .with(Method::POST, Arc::new(OwnershipApi::tax_request)),
            )
            .resource(
                ResourceHandler::new("v1/objects")
                    .with(Method::GET, Arc::new(OwnershipApi::get_objects)),
            )
            .resource(
                ResourceHandler::new("v1/objects/history")
                    .with(Method::GET, Arc::new(OwnershipApi::get_object_history)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/documents")
                    .with(Method::GET, Arc::new(OwnershipApi::get_file))
                    .with(
                        Method::POST,
                        Arc::new(OwnershipApi::attach_contract_other_file),
                    )
                    .with(
                        Method::DELETE,
                        Arc::new(OwnershipApi::delete_contract_files),
                    ),
            )
            .resource(
                ResourceHandler::new("v1/contracts/confirm")
                    .with(Method::POST, Arc::new(OwnershipApi::confirm_contract)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/confirm_create/status").with(
                    Method::GET,
                    Arc::new(OwnershipApi::get_confirm_create_status),
                ),
            )
            .resource(
                ResourceHandler::new("v1/contracts/confirm_create")
                    .with(Method::POST, Arc::new(OwnershipApi::post_confirm_create)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/status")
                    .with(Method::GET, Arc::new(OwnershipApi::get_contract_status)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/refuse")
                    .with(Method::POST, Arc::new(OwnershipApi::refuse_contract)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/sign")
                    .with(Method::POST, Arc::new(OwnershipApi::sign_contract)),
            )
            .resource(
                ResourceHandler::new("v1/esia/token")
                    .with(Method::PUT, Arc::new(OwnershipApi::put_member_token)),
            );

        #[cfg(feature = "internal_api")]
        builder
            .private_scope()
            .web_backend()
            .resource(
                ResourceHandler::new("v1/lots")
                    .with(Method::PUT, Arc::new(OwnershipApi::edit_lot_status)),
            )
            .resource(
                ResourceHandler::new("v1/members")
                    .with(Method::POST, Arc::new(OwnershipApi::add_participant)),
            )
            .resource(
                ResourceHandler::new("v1/objects")
                    .with(Method::POST, Arc::new(OwnershipApi::add_object))
                    .with(Method::PUT, Arc::new(OwnershipApi::update_object)),
            )
            .resource(
                ResourceHandler::new("v1/objects/participates")
                    .with(Method::GET, Arc::new(OwnershipApi::get_object_participates)),
            )
            .resource(
                ResourceHandler::new("v1/lots/bids/publish")
                    .with(Method::POST, Arc::new(OwnershipApi::publish_bids)),
            )
            .resource(
                ResourceHandler::new("v1/lots/execute")
                    .with(Method::POST, Arc::new(OwnershipApi::execute_lot)),
            )
            .resource(
                ResourceHandler::new("v1/lots/undefined")
                    .with(Method::POST, Arc::new(OwnershipApi::undefined)),
            )
            .resource(
                ResourceHandler::new("v1/lots/checks")
                    .with(Method::POST, Arc::new(OwnershipApi::lot_submit_checks)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/documents")
                    .with(Method::GET, Arc::new(OwnershipApi::get_file_private))
                    .with(
                        Method::POST,
                        Arc::new(OwnershipApi::attach_contract_main_file),
                    ),
            )
            .resource(
                ResourceHandler::new("v1/contracts/draft")
                    .with(Method::POST, Arc::new(OwnershipApi::draft_contract)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/approve")
                    .with(Method::POST, Arc::new(OwnershipApi::approve_contract)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/reject")
                    .with(Method::POST, Arc::new(OwnershipApi::reject_contract)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/register")
                    .with(Method::POST, Arc::new(OwnershipApi::register_contract)),
            )
            .resource(ResourceHandler::new("v1/contracts/await_user_action").with(
                Method::POST,
                Arc::new(OwnershipApi::await_user_action_contract),
            ))
            .resource(
                ResourceHandler::new("v1/contracts/checks")
                    .with(Method::POST, Arc::new(OwnershipApi::contract_submit_checks)),
            )
            .resource(ResourceHandler::new("v1/contracts/reference_number").with(
                Method::POST,
                Arc::new(OwnershipApi::contract_reference_number),
            ))
            .resource(
                ResourceHandler::new("v1/contracts/undefined")
                    .with(Method::POST, Arc::new(OwnershipApi::contract_undefined)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/new")
                    .with(Method::POST, Arc::new(OwnershipApi::post_contract_new)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/unconfirm_create")
                    .with(Method::POST, Arc::new(OwnershipApi::post_unconfirm_create)),
            )
            .resource(ResourceHandler::new("v1/tax").with(
                Method::POST,
                Arc::new(OwnershipApi::add_tax_with_payment_details),
            ))
            .resource(
                ResourceHandler::new("v1/tax/status")
                    .with(Method::POST, Arc::new(OwnershipApi::add_tax_status)),
            )
            .resource(ResourceHandler::new("v1/tax/contract/calculation").with(
                Method::POST,
                Arc::new(OwnershipApi::add_tax_contract_calculation),
            ))
            .resource(ResourceHandler::new("v1/tax/lot/calculation").with(
                Method::POST,
                Arc::new(OwnershipApi::add_tax_lot_calculation),
            ))
            .resource(
                ResourceHandler::new("v1/esia/token")
                    .with(Method::GET, Arc::new(OwnershipApi::get_member_token)),
            );

        #[cfg(feature = "extra_counter")]
        builder
            .public_scope()
            .web_backend()
            .resource(
                ResourceHandler::new("v1/objects/counter")
                    .with(Method::GET, Arc::new(OwnershipApi::objects_counter)),
            )
            .resource(
                ResourceHandler::new("v1/lots/counter")
                    .with(Method::GET, Arc::new(OwnershipApi::lots_counter)),
            );

        #[cfg(all(feature = "internal_api", feature = "extra_counter"))]
        builder.private_scope().web_backend().resource(
            ResourceHandler::new("v1/contracts/counter")
                .with(Method::GET, Arc::new(OwnershipApi::contracts_counter)),
        );
    }
}

#[cfg(test)]
mod test {
    use blockp_core::crypto::Hash;
    use std::str::FromStr;

    use crate::data::attachment::DocumentId;
    use crate::data::conditions::ContractType;
    use crate::data::cost::Cost;
    use crate::data::lot::SaleType;
    use crate::data::object::ObjectIdentity;
    use crate::dto::test::{new_check_info, new_conditions_info, new_lot_info};

    use super::*;

    #[test]
    fn get_v1_objects_history() {
        let json = r#"
        {
            "object": {"class":1,"reg_number":"123451"}
        }"#;
        let true_val = GetObjectHistory {
            object: ObjectIdentity::from_str("trademark::123451")
                .unwrap()
                .into(),
        };
        let val: GetObjectHistory = serde_json::from_str(json).unwrap();
        assert_eq!(val, true_val)
    }

    #[test]
    fn post_v1_members() {
        let json = r#"
        {
            "member": {"class":0,"number":"1053600591197"},
            "node": "node_1"
        }"#;
        let true_val = AddParticipant {
            member: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            node: String::from("node_1"),
        };
        let val: AddParticipant = serde_json::from_str(json).unwrap();
        assert_eq!(val, true_val)
    }

    #[test]
    fn post_v1_contracts_acquire_lot() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = AcquireLot {
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };
        let val: AcquireLot = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_lots() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "lot": {
                "name": "My Lot 1",
                "desc": "Explicit lot description",
                "price": 5000000,
                "opening_time": "2020-12-10T02:00:53+00:00",
                "closing_time": "2020-12-31T05:00:53+00:00",
                "objects": ["trademark::123"],
                "status": "undefined"
            },
            "ownership": {
                "contract_type": "license",
                "objects": [{
                    "object": {"class":1,"reg_number":"123451"},
                    "contract_term": { "specification": "forever" },
                    "exclusive": false,
                    "can_distribute": "unable",
                    "location": [{"registry": 1, "code": 45379000,"desc":""}],
                    "classifiers": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}]
                }],
                "payment_conditions": "Condition desc text",
                "payment_comment": null,
                "termination_conditions": ["Term cond 1", "Term cond 2"],
                "contract_extras": ["Extra comment"]
            }
        }"#;
        let requestor = MemberIdentity::from_str("ogrn::1053600591197").unwrap();
        let true_val = OpenLot {
            requestor: requestor.clone().into(),
            lot: new_lot_info(),
            ownership: new_conditions_info(),
        };
        let val: OpenLot = serde_json::from_str(json).unwrap();
        let sale_type = match true_val.ownership.contract_type {
            ContractType::Expropriation | ContractType::Undefined => SaleType::Auction,
            _ => SaleType::PrivateSale,
        };
        assert_eq!(true_val, val);
        val.lot
            .into_lot(requestor, sale_type)
            .unwrap()
            .verify()
            .unwrap();
    }

    #[test]
    fn post_v1_contracts_purchase_offer() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "buyer": {"class":0,"number":"1053600591197"},
            "rightholder": {"class":0,"number":"1053600591197"},
            "price": 100000,
            "conditions": {
                "contract_type": "license",
                "objects": [{
                    "object": {"class":1,"reg_number":"123451"},
                    "contract_term": { "specification": "forever" },
                    "exclusive": false,
                    "can_distribute": "unable",
                    "location": [{"registry": 1, "code": 45379000,"desc":""}],
                    "classifiers": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}]
                }],
                "payment_conditions": "Condition desc text",
                "payment_comment": null,
                "termination_conditions": ["Term cond 1", "Term cond 2"],
                "contract_extras": ["Extra comment"]
            }
        }"#;
        let true_val = PurchaseOffer {
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            buyer: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            rightholder: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            price: Cost::from_str("1000.00").expect("Unable to parse valid Cost"),
            conditions: new_conditions_info(),
        };
        let val: PurchaseOffer = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn put_v1_contracts() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "requestor": {"class":0,"number":"1053600591197"},
            "price": 100000,
            "conditions": {
                "contract_type": "license",
                "objects": [{
                    "object": {"class":1,"reg_number":"123451"},
                    "contract_term": { "specification": "forever" },
                    "exclusive": false,
                    "can_distribute": "unable",
                    "location": [{"registry": 1, "code": 45379000,"desc":""}],
                    "classifiers": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}]
                }],
                "payment_conditions": "Condition desc text",
                "payment_comment": null,
                "termination_conditions": ["Term cond 1", "Term cond 2"],
                "contract_extras": ["Extra comment"]
            }
        }"#;
        let true_val = UpdateContract {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            price: Cost::from_str("1000.00").expect("Unable to parse valid Cost"),
            conditions: new_conditions_info(),
            contract_correspondence: None,
            objects_correspondence: None,
        };

        let val: UpdateContract = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn delete_v1_lots() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = CloseLot {
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };

        let val: CloseLot = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn get_v1_lots() {
        let json = r#"
        {
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = GetLots {
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };

        let val: GetLots = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn put_v1_lots() {
        let json = r#"
        {
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "status": "rejected"
        }"#;
        let true_val = EditLotStatus {
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            status: LotStatus::Rejected,
        };

        let val: EditLotStatus = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_lots_bids() {
        let json = r#"
            {
                "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
                "value": 10023,
                "requestor": {"class":0,"number":"1053600591197"}
            }"#;
        let true_val = AddBid {
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            value: Cost::from(10023),
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
        };

        let val: AddBid = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_lots_bids_publish() {
        let json = r#"
            {
                "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
                "bids": [10023, 12023]
            }"#;
        let true_val = PublishBids {
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            bids: vec![Cost::from(10023), Cost::from(12023)],
        };

        let val: PublishBids = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn delete_v1_contracts_documents() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "doc_tx_hashes": ["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad", "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"]
        }"#;
        let true_val = DeleteContractFiles {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            doc_tx_hashes: vec![
                HashInfo(
                    Hash::from_str(
                        "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
                    )
                    .unwrap(),
                ),
                HashInfo(
                    Hash::from_str(
                        "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
                    )
                    .unwrap(),
                ),
            ],
        };
        let val: DeleteContractFiles = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_confirm() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "deed_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "application_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = ConfirmContract {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            deed_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            application_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
        };
        let val: ConfirmContract = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_refuse() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "reason": "Because"
        }"#;
        let true_val = RefuseContract {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            reason: String::from("Because"),
        };
        let val: RefuseContract = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_draft() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = DraftContract {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };
        let val: DraftContract = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_lots_execute() {
        let json = r#"
        {
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;

        let true_val = ExecuteLot {
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };
        let val: ExecuteLot = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_lots_extend() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "new_expiration": "2020-12-19T16:39:57-08:00"
        }"#;

        let true_val = ExtendLotPeriod {
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            new_expiration: DateTime::<Utc>::from_str("2020-12-19T16:39:57-08:00").unwrap(),
        };
        let val: ExtendLotPeriod = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn delete_v1_documents() {
        let json = r#"
        {
            "requestor": {"class":0,"number":"1053600591197"},
            "doc_tx_hashes": ["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad", "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"]
        }"#;

        let true_val = DeleteFiles {
            requestor: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            doc_tx_hashes: vec![
                HashInfo(
                    DocumentId::from_str(
                        "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
                    )
                    .unwrap(),
                ),
                HashInfo(
                    DocumentId::from_str(
                        "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
                    )
                    .unwrap(),
                ),
            ],
        };

        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    /* turn on when test/resource folder is merged
    #[test]
    fn post_v1_documents_signs() {
        let s = fs::read_to_string("../test/resource/test.xlsx.sig").unwrap();
        let s = s.trim();
        let json = format!(
            "
        {{
            \"requestor\": \"ogrn::1053600591197\",
            \"doc_tx_hash\": \"d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad\",
            \"sign\": \"{}\"
        }}",
            s
        );

        let true_val = AddAttachmentSign {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            doc_tx_hash: HashInfo(Hash::from_str(
                "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            )
            .unwrap()),
            sign: Sign::from_str(&s).unwrap(),
        };
        let val = serde_json::from_str(json.as_str()).unwrap();
        assert_eq!(true_val, val);
    }
     */

    #[test]
    fn post_v1_contracts_register() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;

        let true_val = ContractHash {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };

        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_await_user_action() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;

        let true_val = ContractHash {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };

        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_checks_with_null() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "checks": {
                "seller_data_valid": {
                    "result": "ok",
                    "description": "good"
                }
            },
            "is_undef": false
        }"#;
        let check_info = vec![(CheckKey::SellerDataValid, new_check_info())];
        let true_val = ContractSubmitChecks {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            checks: check_info.into_iter().collect(),
            is_undef: false,
            reference_number: None,
        };

        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_checks_with_ref_numb() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "checks": {
                "seller_data_valid": {
                    "result": "ok",
                    "description": "good"
                }
            },
            "is_undef": false,
            "reference_number": "12345"
        }"#;
        let check_info = vec![(CheckKey::SellerDataValid, new_check_info())];
        let true_val = ContractSubmitChecks {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            checks: check_info.into_iter().collect(),
            is_undef: false,
            reference_number: Some("12345".to_string()),
        };

        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_reference_number() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "reference_number": "number1"
        }"#;
        let true_val = ReferenceContractNumber {
            contract_tx_hash: HashInfo::from_str(
                "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            )
            .unwrap(),
            reference_number: "number1".to_string(),
        };

        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_lots_undefined() {
        let json = r#"
        {
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "admit": true
        }"#;
        let true_val = LotUndefined {
            lot_tx_hash: HashInfo::from_str(
                "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            )
            .unwrap(),
            admit: true,
        };

        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn get_v1_contract_conditions() {
        let json = r#"
        {
            "contract_type": "license",
            "objects": [{
                "object": {"class":1,"reg_number":"123451"},
                "contract_term": { "specification": "forever" },
                "exclusive": false,
                "can_distribute": "unable",
                "location": [{"registry": 1, "code": 45379000,"desc": ""}],
                "classifiers": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}]
            }],
            "payment_conditions": "Condition desc text",
            "payment_comment": null,
            "termination_conditions": ["Term cond 1", "Term cond 2"],
            "contract_extras": ["Extra comment"]
        }
        "#;

        let true_val = new_conditions_info();
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }
}
