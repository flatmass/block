use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use actix_web::http::Method;
use actix_web::HttpMessage;
use chrono::{DateTime, Utc};
use futures::{Future, IntoFuture, Stream};

use blockp_core::api::backends::actix::{FutureResponse, HttpRequest, ResourceHandler};
use blockp_core::api::{ServiceApiBackend, ServiceApiBuilder};
use blockp_core::crypto::Hash;

use crate::control;
use crate::data::conditions::CheckKey;
use crate::data::contract::ContractId;
use crate::data::cost::Cost;
use crate::data::lot::LotStatus;
use crate::data::member::MemberIdentity;
use crate::data::strings::verify_filename;
#[cfg(feature = "internal_api")]
use crate::data::strings::verify_node_name;
use crate::dto::{
    CheckInfo, ConditionsInfo, HashInfo, LotInfo, MemberInfo, ObjectData, ObjectInfo, SignInfo,
};
use crate::error::{Error, FutureResponseError};
use crate::response::IntoResponse;
use crate::schema::Schema;
use crate::upload::handle_multipart_item;
use crate::util::get_from_map;
use crate::util::get_from_multipart_map;
use crate::util::get_slice_from_map;
use crate::util::get_str_from_map;

pub struct OwnershipApi;

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct AddObjectRequest {
    requestor: MemberInfo,
    object: ObjectInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct AddObjectGroupRequest {
    requestor: MemberInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct GetObjectHistory {
    object: ObjectInfo,
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
    rightholder: MemberInfo,
    price: Cost,
    conditions: ConditionsInfo,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct AddTax {
    contract_tx_hash: HashInfo,
    requestor: MemberInfo,
    payment_number: String,
    payment_date: DateTime<Utc>,
    amount: Cost,
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
    doc_tx_hashes: Vec<HashInfo>,
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
    doc_tx_hashes: Vec<HashInfo>,
    deed_tx_hash: HashInfo,
    application_tx_hash: HashInfo,
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
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ExecuteLot {
    lot_tx_hash: HashInfo,
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
struct SubmitChecks {
    contract_tx_hash: HashInfo,
    checks: HashMap<CheckKey, CheckInfo>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
struct ReferenceContractNumber {
    contract_tx_hash: HashInfo,
    reference_number: String,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ObjectsCounter {
    pub objects: usize,
}

impl OwnershipApi {
    fn add_object_request(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(move |AddObjectRequest { requestor, object }| {
                control::add_object_request(state, requestor.into(), object.into())
            })
            .into_response()
    }

    fn add_object_group_request(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(move |AddObjectGroupRequest { requestor }| {
                control::add_object_group_request(state, requestor.into())
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn add_object(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .map_err(|_e| Error::bad_content_type())
            .collect()
            .and_then(move |params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let member = get_from_multipart_map(&params, "owner")?;
                let object = get_from_multipart_map(&params, "object")?;
                let data = get_str_from_map(&params, "data")?;
                let ownership_str = get_str_from_map(&params, "ownership")?;
                let ownership = serde_json::from_str(ownership_str)
                    .map_err(|e| Error::bad_json(ownership_str, e))?;
                control::add_object(state, member, object, data, ownership)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn update_object(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .map_err(|_e| Error::bad_content_type())
            .collect()
            .and_then(move |params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let member = get_from_multipart_map(&params, "owner")?;
                let object = get_from_multipart_map(&params, "object")?;
                let data = get_str_from_map(&params, "data")?;
                let ownership_str = get_str_from_map(&params, "ownership")?;
                let ownership = serde_json::from_str(ownership_str)
                    .map_err(|e| Error::bad_json(ownership_str, e))?;
                control::update_object(state, member, object, data, ownership)
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
                control::open_lot(
                    state,
                    params.requestor.into(),
                    params.lot,
                    params.ownership.into(),
                )
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
        if req.content_type() == "" {
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
            } else {
                control::get_all_lots(state).into_future().into_response()
            }
        } else {
            let res: Result<(), Error> = Err(Error::bad_content_type());
            let fut = res.into_future();
            fut.into_response()
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
                if params.status != LotStatus::Rejected && params.status != LotStatus::Verified {
                    Error::bad_state("lot status should be 'rejected' or 'verified'").ok()?
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
                    params.rightholder.into(),
                    params.price.into(),
                    params.conditions.into(),
                )
            })
            .into_response()
    }

    fn add_tax(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(move |params: AddTax| {
                if params.payment_number.is_empty() {
                    Error::empty_param("payment_number").ok()?
                }
                control::add_tax(
                    state,
                    params.contract_tx_hash.into(),
                    params.requestor.into(),
                    params.payment_number.into(),
                    params.payment_date,
                    params.amount.into(),
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
                control::draft_contract(
                    state,
                    &params.contract_tx_hash,
                    &params
                        .doc_tx_hashes
                        .into_iter()
                        .map(|v| v.into())
                        .collect::<Vec<Hash>>(),
                    &params.deed_tx_hash,
                    &params.application_tx_hash,
                )
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
                .and_then(|object| control::get_object(state, object).map(ObjectData::from))
                .into_response()
        } else {
            Error::empty_param("object").error_future_response()
        }
    }

    fn attach_file(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .map_err(|_e| Error::bad_content_type())
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let mut members = vec![];
                for (key, value) in params.iter() {
                    if key == "members" {
                        if value.is_empty() {
                            Error::empty_param("members").ok()?
                        }
                        let value = String::from_utf8(value.clone()).map_err(Error::from)?;
                        let member = MemberIdentity::from_str(value.as_str())?;
                        members.push(member);
                    }
                }
                let params = params.into_iter().collect();
                let requestor = get_from_multipart_map(&params, "requestor")?;
                let data = get_slice_from_map(&params, "file")?;
                let name = get_str_from_map(&params, "name")?;
                let file_type = get_from_multipart_map(&params, "file_type")?;
                verify_filename(name)?;
                control::attach_file(state, requestor, name, data, file_type, members)
            })
            .into_response()
    }

    fn delete_files(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(
                move |DeleteFiles {
                          requestor,
                          doc_tx_hashes,
                      }| {
                    control::delete_files(
                        state,
                        requestor.into(),
                        &doc_tx_hashes
                            .into_iter()
                            .map(|v| v.into())
                            .collect::<Vec<Hash>>(),
                    )
                },
            )
            .into_response()
    }

    fn add_attachment_sign(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(
                move |AddAttachmentSign {
                          requestor,
                          doc_tx_hash,
                          sign,
                      }| {
                    control::add_attachment_sign(state, requestor.into(), &doc_tx_hash, sign.into())
                },
            )
            .into_response()
    }

    fn get_file(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        let requestor = get_from_map(&query, "requestor");
        get_from_map(&query, "doc_tx_hash")
            .into_future()
            .and_then(|doc_tx_hash: HashInfo| control::get_file(state, requestor?, &doc_tx_hash))
            .into_response()
    }

    fn attach_contract_file(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .map_err(|_e| Error::bad_content_type())
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let requestor = get_from_multipart_map(&params, "requestor")?;
                let contract_tx_hash = get_str_from_map(&params, "contract_tx_hash")?.parse()?;
                let file = get_slice_from_map(&params, "file")?;
                let name = get_str_from_map(&params, "name")?;
                let file_type = get_from_multipart_map(&params, "file_type")?;
                control::attach_contract_file(
                    state,
                    requestor,
                    name,
                    &contract_tx_hash,
                    file,
                    file_type,
                )
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
                    &params
                        .doc_tx_hashes
                        .into_iter()
                        .map(|v| v.into())
                        .collect::<Vec<Hash>>(),
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
            .map_err(|_e| Error::bad_content_type())
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let contract_tx_hash = get_str_from_map(&params, "contract_tx_hash")?.parse()?;
                let data = get_slice_from_map(&params, "file")?;
                let name = get_str_from_map(&params, "name")?;
                verify_filename(name)?;
                let sign = get_from_multipart_map(&params, "sign")?;
                control::approve_contract(state, &contract_tx_hash, name, data, sign)
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

    #[cfg(feature = "internal_api")]
    fn reject_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.multipart()
            .map(handle_multipart_item)
            .flatten()
            .map_err(|_e| Error::bad_content_type())
            .collect()
            .and_then(|params: Vec<(String, Vec<u8>)>| {
                let params = params.into_iter().collect();
                let contract_tx_hash = get_str_from_map(&params, "contract_tx_hash")?.parse()?;
                let data = get_slice_from_map(&params, "file").ok();
                let name = get_str_from_map(&params, "name").ok();
                let reason = get_str_from_map(&params, "reason")?;
                let sign = get_from_multipart_map(&params, "sign").ok();
                control::reject_contract(state, &contract_tx_hash, reason, name, data, sign)
            })
            .into_response()
    }

    fn update_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .map_err(|_| Error::bad_content_type())
            .and_then(|params: UpdateContract| {
                control::update_contract(
                    state,
                    &params.contract_tx_hash,
                    params.requestor.into(),
                    params.price.into(),
                    params.conditions.into(),
                )
            })
            .into_response()
    }

    fn get_contract(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let query = req.query();
        get_from_map(&query, "contract_tx_hash")
            .into_future()
            .and_then(|contract_tx_hash: ContractId| {
                control::get_contract(state, &contract_tx_hash)
            })
            .into_response()
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
    fn submit_checks(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: SubmitChecks| {
                control::submit_checks(state, &params.contract_tx_hash, params.checks)
            })
            .into_response()
    }

    #[cfg(feature = "internal_api")]
    fn contract_reference_number(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        req.json()
            .from_err()
            .and_then(|params: ReferenceContractNumber| {
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

    fn objects_counter(req: HttpRequest) -> FutureResponse {
        let state = req.state().clone();
        let schema = Schema::new(state.snapshot());
        let json = ObjectsCounter {
            objects: schema.objects().keys().count().into(),
        };
        futures::future::ok::<ObjectsCounter, Error>(json).into_response()
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
                ResourceHandler::new("v1/contracts/tax")
                    .with(Method::POST, Arc::new(OwnershipApi::add_tax)),
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
                ResourceHandler::new("v1/objects/request")
                    .with(Method::POST, Arc::new(OwnershipApi::add_object_request)),
            )
            .resource(ResourceHandler::new("v1/objects/request/member").with(
                Method::POST,
                Arc::new(OwnershipApi::add_object_group_request),
            ))
            .resource(
                ResourceHandler::new("v1/documents")
                    .with(Method::POST, Arc::new(OwnershipApi::attach_file))
                    .with(Method::DELETE, Arc::new(OwnershipApi::delete_files))
                    .with(Method::GET, Arc::new(OwnershipApi::get_file)),
            )
            .resource(
                ResourceHandler::new("v1/documents/signs")
                    .with(Method::POST, Arc::new(OwnershipApi::add_attachment_sign)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/documents")
                    .with(Method::POST, Arc::new(OwnershipApi::attach_contract_file))
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
                ResourceHandler::new("v1/objects/counter")
                    .with(Method::GET, Arc::new(OwnershipApi::objects_counter)),
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
                ResourceHandler::new("v1/lots/bids/publish")
                    .with(Method::POST, Arc::new(OwnershipApi::publish_bids)),
            )
            .resource(
                ResourceHandler::new("v1/lots/execute")
                    .with(Method::POST, Arc::new(OwnershipApi::execute_lot)),
            )
            .resource(
                ResourceHandler::new("v1/contracts/documents")
                    .with(Method::POST, Arc::new(OwnershipApi::attach_contract_file)),
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
                    .with(Method::POST, Arc::new(OwnershipApi::submit_checks)),
            )
            .resource(ResourceHandler::new("v1/contracts/reference_number").with(
                Method::POST,
                Arc::new(OwnershipApi::contract_reference_number),
            ));
    }
}

#[cfg(test)]
mod test {
    use blockp_core::crypto::Hash;

    use crate::data::attachment::DocumentId;
    use crate::data::cost::Cost;
    use crate::data::object::ObjectIdentity;
    use crate::dto::test::{new_check_info, new_conditions_info, new_lot_info};

    use super::*;

    #[test]
    fn post_v1_objects_request() {
        let json = r#"
        {
            "requestor": "ogrn::1053600591197",
            "object": "trademark::123451"
        }"#;
        let true_val = AddObjectRequest {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            object: ObjectInfo(ObjectIdentity::from_str("trademark::123451").unwrap()),
        };
        let val: AddObjectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(val, true_val)
    }

    #[test]
    fn post_v1_objects_request_member() {
        let json = r#"
        {
            "requestor": "ogrn::1053600591197"
        }"#;
        let true_val = AddObjectGroupRequest {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
        };
        let val: AddObjectGroupRequest = serde_json::from_str(json).unwrap();
        assert_eq!(val, true_val)
    }

    #[test]
    fn get_v1_objects_history() {
        let json = r#"
        {
            "object": "trademark::123451"
        }"#;
        let true_val = GetObjectHistory {
            object: ObjectInfo(ObjectIdentity::from_str("trademark::123451").unwrap()),
        };
        let val: GetObjectHistory = serde_json::from_str(json).unwrap();
        assert_eq!(val, true_val)
    }

    #[test]
    fn post_v1_members() {
        let json = r#"
        {
            "member": "ogrn::1053600591197",
            "node": "node_1"
        }"#;
        let true_val = AddParticipant {
            member: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            node: String::from("node_1"),
        };
        let val: AddParticipant = serde_json::from_str(json).unwrap();
        assert_eq!(val, true_val)
    }

    #[test]
    fn post_v1_contracts_acquire_lot() {
        let json = r#"
        {
            "requestor": "ogrn::1053600591197",
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = AcquireLot {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
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
            "requestor": "ogrn::1053600591197",
            "lot": {
                "name": "My Lot 1",
                "desc": "Explicit lot description",
                "price": 5000000,
                "sale_type": "auction",
                "opening_time": "2020-12-10T02:00:53+00:00",
                "closing_time": "2020-12-31T05:00:53+00:00",
                "objects": ["trademark::123"],
                "status": "undefined"
            },
            "ownership": {
                "contract_type": "license",
                "objects": [{
                    "object": "trademark::123451",
                    "contract_term": { "specification": "forever" },
                    "exclusive": false,
                    "can_distribute": "unable",
                    "location": ["oktmo::45379000"],
                    "classifiers": ["mktu::8", "mktu::13"]
                }],
                "payment_conditions": "Condition desc text",
                "payment_comment": null,
                "termination_conditions": ["Term cond 1", "Term cond 2"],
                "contract_extras": ["Extra comment"]
            }
        }"#;
        let true_val = OpenLot {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            lot: new_lot_info(),
            ownership: new_conditions_info(),
        };
        let val: OpenLot = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
        val.lot.into_lot().unwrap().verify().unwrap();
    }

    #[test]
    fn post_v1_contracts_purchase_offer() {
        let json = r#"
        {
            "requestor": "ogrn::1053600591197",
            "rightholder": "ogrn::1053600591197",
            "price": 100000,
            "conditions": {
                "contract_type": "license",
                "objects": [{
                    "object": "trademark::123451",
                    "contract_term": { "specification": "forever" },
                    "exclusive": false,
                    "can_distribute": "unable",
                    "location": ["oktmo::45379000"],
                    "classifiers": ["mktu::8", "mktu::13"]
                }],
                "payment_conditions": "Condition desc text",
                "payment_comment": null,
                "termination_conditions": ["Term cond 1", "Term cond 2"],
                "contract_extras": ["Extra comment"]
            }
        }"#;
        let true_val = PurchaseOffer {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            rightholder: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            price: Cost::from_str("1000.00").expect("Unable to parse valid Cost"),
            conditions: new_conditions_info(),
        };
        let val: PurchaseOffer = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_tax() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "requestor": "ogrn::1053600591197",
            "payment_number": "payment_id",
            "payment_date": "2020-12-19T16:39:57-08:00",
            "amount": 100000
        }"#;
        let true_val = AddTax {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            payment_number: String::from("payment_id"),
            payment_date: DateTime::<Utc>::from_str("2020-12-19T16:39:57-08:00").unwrap(),
            amount: Cost::from_str("1000.00").expect("Unable to parse valid Cost"),
        };
        let val: AddTax = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn put_v1_contracts() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "requestor": "ogrn::1053600591197",
            "price": 100000,
            "conditions": {
                "contract_type": "license",
                "objects": [{
                    "object": "trademark::123451",
                    "contract_term": { "specification": "forever" },
                    "exclusive": false,
                    "can_distribute": "unable",
                    "location": ["oktmo::45379000"],
                    "classifiers": ["mktu::8", "mktu::13"]
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
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            price: Cost::from_str("1000.00").expect("Unable to parse valid Cost"),
            conditions: new_conditions_info(),
        };

        let val: UpdateContract = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn delete_v1_lots() {
        let json = r#"
        {
            "requestor": "ogrn::1053600591197",
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = CloseLot {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
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
                "requestor": "ogrn::1053600591197"
            }"#;
        let true_val = AddBid {
            lot_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            value: Cost::from(10023),
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
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
            "requestor": "ogrn::1053600591197",
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "doc_tx_hashes": ["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad", "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"]
        }"#;
        let true_val = DeleteContractFiles {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
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
            "requestor": "ogrn::1053600591197",
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "deed_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "application_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "doc_tx_hashes": ["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad", "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"]
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
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
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
        let val: ConfirmContract = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_refuse() {
        let json = r#"
        {
            "requestor": "ogrn::1053600591197",
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "reason": "Because"
        }"#;
        let true_val = RefuseContract {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
            reason: String::from("Because"),
        };
        let val: RefuseContract = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn post_v1_contracts_draft() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "doc_tx_hashes": ["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad", "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"],
            "deed_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "application_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;
        let true_val = DraftContract {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
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
            deed_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            application_tx_hash: HashInfo(
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
            "requestor": "ogrn::1053600591197",
            "lot_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "new_expiration": "2020-12-19T16:39:57-08:00"
        }"#;

        let true_val = ExtendLotPeriod {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
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
            "requestor": "ogrn::1053600591197",
            "doc_tx_hashes": ["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad", "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"]
        }"#;

        let true_val = DeleteFiles {
            requestor: MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap()),
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
    fn post_v1_contracts_checks() {
        let json = r#"
        {
            "contract_tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad",
            "checks": {
                "seller_data_valid": {
                    "result": "ok",
                    "description": "good"
                }
            }
        }"#;
        let check_info = vec![(CheckKey::SellerDataValid, new_check_info())];
        let true_val = SubmitChecks {
            contract_tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
            checks: check_info.into_iter().collect(),
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
    fn get_v1_contract_conditions() {
        let json = r#"
        {
            "contract_type": "license",
            "objects": [{
                "object": "trademark::123451",
                "contract_term": { "specification": "forever" },
                "exclusive": false,
                "can_distribute": "unable",
                "location": ["oktmo::45379000"],
                "classifiers": ["mktu::8", "mktu::13"]
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
