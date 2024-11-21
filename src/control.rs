use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use chrono::{DateTime, Utc};

use blockp_core::api::ServiceApiState as State;
use blockp_core::blockchain::Transaction;
use blockp_core::crypto::{Hash, PublicKey};
use blockp_core::node::{TransactionSend, TransactionSendPrivate};
use blockp_core::storage::Snapshot;

use crate::data::attachment::{Attachment, AttachmentType, DocumentId, Sign};
#[cfg(feature = "internal_api")]
use crate::data::conditions::Check;
use crate::data::conditions::{CheckKey, Conditions, ContractType};
#[cfg(feature = "internal_api")]
use crate::data::contract::Action;
use crate::data::contract::{ContractId, ContractStatus, CorrespondenceContacts};
use crate::data::cost::Cost;
use crate::data::lot::{LotId, LotStatus, SaleType};
use crate::data::member::MemberIdentity;
use crate::data::object::ObjectIdentity;
#[cfg(feature = "internal_api")]
use crate::data::ownership::{Ownership, OwnershipUnstructured};
#[cfg(feature = "internal_api")]
use crate::data::payment::PaymentStatus;
#[cfg(feature = "internal_api")]
use crate::data::payment::{Calculation, PaymentDetail};
use crate::dto::*;
use crate::error::{Error, Result};
use crate::schema::Schema;
use crate::transactions::{self, get_private_tx, get_transaction, OwnershipTransactions};

#[cfg(feature = "internal_api")]
fn split_ownership(
    ownership: Vec<OwnershipInfo>,
) -> Result<(Vec<Ownership>, Vec<OwnershipUnstructured>)> {
    ownership.into_iter().try_fold(
        (Vec::new(), Vec::new()),
        |(mut structured, mut unstructured), ownership| {
            match ownership {
                OwnershipInfo::Structured(info) => structured.push(info.try_into()?),
                OwnershipInfo::Unstructured(info) => unstructured.push(info.try_into()?),
            }
            Ok((structured, unstructured))
        },
    )
}

#[cfg(feature = "internal_api")]
pub fn add_object(
    state: State,
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<OwnershipInfo>,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let (structured_ownership, unstructured_ownership) = split_ownership(ownership)?;
    let tx = transactions::add_object(
        object,
        data,
        structured_ownership,
        unstructured_ownership,
        cert,
    );
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn update_object(
    state: State,
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<OwnershipInfo>,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let (structured_ownership, unstructured_ownership) = split_ownership(ownership)?;
    let tx = transactions::update_object(
        object,
        data,
        structured_ownership,
        unstructured_ownership,
        cert,
    );
    send(state, tx)
}

pub fn object_history(state: State, object: ObjectIdentity) -> Result<TxList> {
    let state = Schema::new(state.snapshot());

    let history = state.object_history(&object.id());
    let history = history.iter().map(|e| e.tx_hash().to_string()).collect();
    Ok(TxList(history))
}

// pub fn attach_file(
//     state: State,
//     requestor: MemberIdentity,
//     name: &str,
//     blob: &[u8],
//     file_type: AttachmentType,
//     members: Vec<MemberIdentity>,
// ) -> Result<TxHash> {
//     let cert = state.blockchain().certificate();
//     let file = Attachment::new(name, blob, file_type as u8);
//     let schema = Schema::new(state.snapshot());
//     let members = dedup_naive(members);
//
//     let share = members
//         .iter()
//         .chain(std::iter::once(&requestor))
//         .flat_map(|p| {
//             schema
//                 .participants(&p.id())
//                 .into_iter()
//                 .collect::<Vec<String>>()
//         })
//         .map(|s| PublicKey::from_slice(s.as_bytes()).ok_or(Error::bad_stored_member(s.as_str())))
//         .collect::<Result<Vec<PublicKey>>>()?;
//
//     let share = dedup_naive(share);
//
//     let tx = transactions::attach_file(requestor, file, cert, members, share);
//     send_private(state, tx)
// }

// pub fn delete_files(
//     state: State,
//     requestor: MemberIdentity,
//     doc_tx_hashes: &[DocumentId],
// ) -> Result<TxHash> {
//     let cert = state.blockchain().certificate();
//     let tx = transactions::delete_files(requestor, doc_tx_hashes, cert);
//     send_private(state, tx)
// }

pub fn get_file<'a>(
    state: State,
    requestor: Option<&'a MemberIdentity>,
    doc_tx_hash: &'a Hash,
) -> Result<AttachmentDto> {
    let schema = Schema::new(state.snapshot());

    let tx = get_private_tx(&schema, doc_tx_hash)?;
    let (contract_tx, attachment) = match &tx {
        OwnershipTransactions::AttachContractMainFile(doc_tx) => {
            let mut attachment = doc_tx.file();
            if doc_tx.file().sign().is_none() {
                if let Some(contract_sign) = schema.get_sign_contract_tx(doc_tx_hash) {
                    let buyer_sign = contract_sign
                        .buyer_sign_tx_hash()
                        .and_then(|sign_info| {
                            get_private_tx(&schema, &sign_info.sign_tx_hash()).ok()
                        })
                        .and_then(|sign_tx| match sign_tx {
                            OwnershipTransactions::SignContract(tx_body) => Some(tx_body),
                            _ => None,
                        })
                        .zip(doc_tx.file().metadata().file_type().try_into().ok())
                        .and_then(|(tx_body, attachment_type)| match attachment_type {
                            AttachmentType::Deed => Some(tx_body.deed_sign()),
                            AttachmentType::Application => Some(tx_body.application_sign()),
                            _ => None,
                        });

                    let seller_sign = contract_sign
                        .seller_sign_tx_hash()
                        .and_then(|sign_info| {
                            get_private_tx(&schema, &sign_info.sign_tx_hash()).ok()
                        })
                        .and_then(|sign_tx| match sign_tx {
                            OwnershipTransactions::SignContract(tx_body) => Some(tx_body),
                            _ => None,
                        })
                        .zip(doc_tx.file().metadata().file_type().try_into().ok())
                        .and_then(|(tx_body, attachment_type)| match attachment_type {
                            AttachmentType::Deed => Some(tx_body.deed_sign()),
                            AttachmentType::Application => Some(tx_body.application_sign()),
                            _ => None,
                        });
                    attachment = Attachment::new(
                        attachment.metadata(),
                        attachment.data(),
                        buyer_sign.clone(),
                    );
                    let mut attachment_dto: AttachmentDto = attachment.try_into()?;
                    attachment_dto.buyer_sign = buyer_sign.map(Into::into);
                    attachment_dto.seller_sign = seller_sign.map(Into::into);
                    return Ok(attachment_dto);
                } else {
                    if let Some(sign_tx_hash) = schema.deprecated_get_sign_contract_tx(doc_tx_hash)
                    {
                        let sign = get_private_tx(&schema, &sign_tx_hash)
                            .ok()
                            .and_then(|sign_tx| match sign_tx {
                                OwnershipTransactions::SignContract(tx_body) => Some(tx_body),
                                _ => None,
                            })
                            .zip(doc_tx.file().metadata().file_type().try_into().ok())
                            .and_then(|(tx_body, attachment_type)| match attachment_type {
                                AttachmentType::Deed => Some(tx_body.deed_sign()),
                                AttachmentType::Application => Some(tx_body.application_sign()),
                                _ => None,
                            });
                        attachment = Attachment::new(attachment.metadata(), attachment.data(), sign)
                    };
                }
            };

            (doc_tx.contract_tx_hash(), attachment)
        }
        OwnershipTransactions::AttachContractOtherFile(doc_tx) => {
            (doc_tx.contract_tx_hash(), doc_tx.file())
        }
        OwnershipTransactions::ApproveContract(doc_tx) => {
            let attachment = doc_tx
                .attachment()
                .ok_or_else(|| Error::no_attachment(doc_tx_hash))?;
            (doc_tx.contract_tx_hash(), attachment)
        }
        OwnershipTransactions::RejectContract(doc_tx) => {
            let attachment = doc_tx
                .attachment()
                .ok_or_else(|| Error::no_attachment(doc_tx_hash))?;
            (doc_tx.contract_tx_hash(), attachment)
        }
        _ => Error::unexpected_tx_type(doc_tx_hash).ok()?,
    };
    let contract = schema
        .contracts()
        .get(contract_tx)
        .ok_or_else(|| Error::no_contract(contract_tx))?;
    if requestor.is_some() && !contract.is_member(requestor.unwrap()) {
        Err(Error::no_permissions())?
    };
    Ok(attachment.try_into()?)
}

// pub fn add_attachment_sign(
//     state: State,
//     requestor_id: MemberIdentity,
//     doc_tx_hash: &Hash,
//     sign: Sign,
// ) -> Result<TxHash> {
//     let cert = state.blockchain().certificate();
//     let schema = Schema::new(state.snapshot());
//
//     let txset = get_private_tx(&schema, doc_tx_hash)?;
//     let share = match txset {
//         OwnershipTransactions::AttachFile(doc_tx) => Ok(doc_tx.share()),
//         _ => Error::unexpected_tx_type(doc_tx_hash).ok(),
//     }?;
//
//     let tx = transactions::add_attachment_sign(requestor_id, doc_tx_hash, sign, cert, share);
//     send_private(state, tx)
// }

pub fn open_lot(
    state: State,
    requestor: MemberIdentity,
    info: LotInfo,
    conditions: ConditionsInfo,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let sale_type = match conditions.contract_type {
        ContractType::Expropriation => SaleType::Auction,
        ContractType::PledgeAgreement => SaleType::PrivateSale,
        _ => {
            if conditions
                .objects
                .iter()
                .any(|object| object.is_exclusive())
            {
                SaleType::Auction
            } else {
                SaleType::PrivateSale
            }
        }
    };
    let lot = info.into_lot(requestor.clone(), sale_type)?;
    lot.verify()?;
    let tx = transactions::open_lot(requestor, lot, conditions.into(), cert);
    send(state, tx)
}

pub fn close_lot(state: State, requestor: MemberIdentity, lot_id: &LotId) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::close_lot(requestor, &lot_id, cert);
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn edit_lot_status(state: State, lot_id: &LotId, lot_status: LotStatus) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::edit_lot_status(lot_id, cert, lot_status);
    send(state, tx)
}

pub fn extend_lot_period(
    state: State,
    requestor: MemberIdentity,
    lot_id: &LotId,
    new_expiration_date: DateTime<Utc>,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::extend_lot_period(requestor, lot_id, new_expiration_date, cert);
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn execute_lot(state: State, lot_id: &LotId) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::execute_lot(lot_id, cert);
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn lot_undefined(state: State, lot_id: &LotId, admit: bool) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::lot_undefined(lot_id, admit, cert);
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn contract_undefined(state: State, contract_id: &ContractId, admit: bool) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let schema = Schema::new(state.snapshot());
    let share = schema.get_contract_share(contract_id)?;
    let tx = transactions::contract_undefined(contract_id, admit, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn draft_contract(state: State, contract_tx_hash: &ContractId) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();
    let share = schema.get_contract_share(contract_tx_hash)?;
    let tx = transactions::draft_contract(contract_tx_hash, share, cert);
    send_private(state, tx)
}

pub fn acquire_lot(state: State, requestor: MemberIdentity, lot_id: &LotId) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let lot_tx = match get_transaction(&schema, lot_id) {
        Ok(OwnershipTransactions::OpenLot(tx)) => Ok(tx),
        _ => Error::no_transaction(lot_id).ok(),
    }?;

    if schema
        .lot_states()
        .get(lot_id)
        .ok_or_else(|| Error::no_lot(lot_id))?
        .undefined()
    {
        Error::lot_is_undefined(lot_id).ok()?;
    }

    let owner = lot_tx.requestor();
    let share = [&owner, &requestor]
        .iter()
        .flat_map(|p| {
            schema
                .participants(&p.id())
                .into_iter()
                .collect::<Vec<String>>()
        })
        .map(|s| {
            PublicKey::from_slice(s.as_bytes()).ok_or_else(|| Error::bad_stored_member(s.as_str()))
        })
        .collect::<Result<Vec<PublicKey>>>()?;

    let tx = transactions::acquire_lot(requestor, &lot_id, share, cert);
    send_private(state, tx)
}

pub fn purchase_offer(
    state: State,
    requestor: MemberIdentity,
    buyer: MemberIdentity,
    rightholder: MemberIdentity,
    price: Cost,
    conditions: Conditions,
) -> Result<TxHash> {
    if &buyer == &rightholder {
        Error::buyer_is_rightholder(&buyer, &rightholder).ok()?;
    }

    if requestor != buyer && requestor != rightholder {
        Error::no_permissions().ok()?;
    };

    let schema = Schema::new(state.snapshot());
    let share = [&rightholder, &buyer]
        .iter()
        .flat_map(|p| {
            schema
                .participants(&p.id())
                .into_iter()
                .collect::<Vec<String>>()
        })
        .map(|s| {
            PublicKey::from_slice(s.as_bytes()).ok_or_else(|| Error::bad_stored_member(s.as_str()))
        })
        .collect::<Result<Vec<PublicKey>>>()?;
    let cert = state.blockchain().certificate();
    let tx = transactions::purchase_offer(
        requestor,
        buyer,
        rightholder,
        price,
        conditions,
        share,
        cert,
    );
    send_private(state, tx)
}

pub fn add_bid(state: State, member: MemberIdentity, lot_id: &LotId, bid: Cost) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::add_bid(member, lot_id, bid.into(), cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn publish_bids(state: State, lot_id: &LotId, bids: Vec<Cost>) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let bids = bids.into_iter().map(|value| value.into()).collect();
    let tx = transactions::publish_bids(lot_id, bids, cert);
    send(state, tx)
}
// Depreceated in FIPSOP-266
// pub fn get_lot_info(state: State, lot_id: &LotId) -> Result<LotInfo> {
//     let schema = Schema::new(state.snapshot());
//     let state = schema
//         .lot_states()
//         .get(lot_id)
//         .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;
//     let price = Cost::from(state.price());
//     let status_val = state.status();
//     let status = LotStatus::try_from(status_val)
//         .map_err(|_| Error::bad_lot_status(&status_val.to_string()))?;
//
//     schema
//         .lots()
//         .get(&lot_id)
//         .ok_or_else(|| Error::no_lot(&lot_id))
//         .map(|lot| LotInfo::from(&lot).set_price(price))
// }

pub fn get_lot_info_with_objects(state: State, lot_id: &LotId) -> Result<LotInfoWithObjects> {
    let schema = Schema::new(state.snapshot());

    let state = schema
        .lot_states()
        .get(lot_id)
        .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;

    let price = Cost::from(state.price());

    let status = LotStatus::try_from(state.status())
        .map_err(|_| Error::bad_lot_status(&state.status().to_string()))?;

    let lot = schema
        .lots()
        .get(lot_id)
        .ok_or_else(|| Error::no_lot(lot_id))?;

    let lot_conditions = schema
        .lot_conditions()
        .get(lot_id)
        .ok_or_else(|| Error::no_lot(lot_id))?;

    let lot_calculations = schema
        .get_lot_calculations(lot_id)
        .into_iter()
        .map(|v| v.into())
        .collect();

    let ref_number = schema.lot_reference_number(lot_id);

    let lot_info_with_objects = LotInfoWithObjects {
        name: lot.name().to_owned(),
        desc: lot.desc().to_owned(),
        seller: lot.seller().into(),
        price,
        sale_type: SaleType::try_from(lot.sale_type())
            .map_err(|_| Error::internal_bad_struct("sale_type"))?,
        opening_time: lot.opening_time(),
        closing_time: lot.closing_time(),
        status,
        is_undefined: state.undefined(),
        conditions: lot_conditions.try_into()?,
        calculations: lot_calculations,
        reference_number: ref_number,
    };
    Ok(lot_info_with_objects)
}

pub fn get_member_lots(state: State, member: MemberIdentity) -> Result<Lots> {
    let schema = Schema::new(state.snapshot());
    let member_id = &member.id();
    Ok(schema
        .member_lots(member_id)
        .into_iter()
        .map(|(_, lot_id)| lot_id)
        .collect())
}

pub fn get_member_contracts(state: State, member: MemberIdentity) -> Result<Vec<ContractId>> {
    let schema = Schema::new(state.snapshot());
    let member_id = &member.id();
    Ok(schema
        .member_contracts(member_id)
        .iter()
        .map(|(contract_id, _)| contract_id)
        .collect())
}

pub fn get_all_lots(state: State) -> Result<Lots> {
    let schema = Schema::new(state.snapshot());
    Ok(schema.lots().keys().collect())
}

pub fn get_lots_pagination(
    state: State,
    limit: usize,
    from: Option<Hash>,
) -> PaginationPage<HashWrapperDto<LotInfoWithObjects>, Option<Hash>> {
    let schema = Schema::new(state.snapshot());
    let iter = schema.lots_list();
    let tmp = if let Some(hash) = from {
        iter.iter_rev()
            .skip_while(|v| v != &hash)
            .take(limit)
            .filter_map(|lot_id| {
                get_lot_info_with_objects(state.clone(), &lot_id)
                    .map(|data| HashWrapperDto::into_hash_wrapper(data, lot_id))
                    .ok()
            })
            .collect()
    } else {
        iter.iter_rev()
            .take(limit)
            .filter_map(|lot_id| {
                get_lot_info_with_objects(state.clone(), &lot_id)
                    .map(|data| HashWrapperDto::into_hash_wrapper(data, lot_id))
                    .ok()
            })
            .collect()
    };
    PaginationPage {
        data: tmp,
        limit,
        from,
    }
}

pub fn get_bids(state: State, lot_id: LotId) -> Result<Vec<Cost>> {
    let schema = Schema::new(state.snapshot());
    Ok(schema.bids(&lot_id).iter().map(Cost::from).collect())
}

pub fn get_bid_transactions(state: State, lot_id: LotId) -> Result<TxList> {
    let schema = Schema::new(state.snapshot());
    Ok(TxList(
        schema
            .bid_history(&lot_id)
            .into_iter()
            .map(|tx_hash| tx_hash.to_string())
            .collect::<Vec<String>>(),
    ))
}

pub fn get_member_objects(state: State, member: MemberIdentity) -> Result<Vec<ObjectIdentityDto>> {
    let schema = Schema::new(state.snapshot());
    Ok(schema
        .ownership(&member.id())
        .into_iter()
        .map(|(_hash, object)| object.into())
        .collect())
}

pub fn get_object(state: State, object: ObjectIdentity) -> Result<ObjectInformationDto> {
    let schema = Schema::new(state.snapshot());
    let object_id = object.id();
    let data = schema
        .objects()
        .get(&object_id)
        .ok_or_else(|| Error::no_object(&object))?;
    let unstructured_ownership: Vec<UnstructuredOwnershipInfo> = schema
        .ownership_unstructured(&object_id)
        .iter()
        .map(|v| v.into())
        .collect();
    let ownership: Vec<StructuredOwnershipInfo> = schema
        .rightholders(&object_id)
        .iter()
        .filter_map(|(rightholder, rights)| {
            StructuredOwnershipInfo::from_rights(rights, rightholder).ok()
        })
        .collect();

    Ok(ObjectInformationDto {
        object: object.into(),
        data,
        ownership,
        unstructured_ownership,
    })
}

pub fn get_objects_pagination(
    state: State,
    limit: usize,
    from: Option<ObjectIdentityDto>,
) -> PaginationPage<ObjectInformationDto, Option<ObjectIdentityDto>> {
    let schema = Schema::new(state.snapshot());

    let iter = schema.objects_list();
    let tmp: Vec<ObjectInformationDto> = if let Some(ref object) = from {
        iter.iter_rev()
            .skip_while(|v| v != &ObjectIdentity::from(object.clone()).id())
            .take(limit)
            .filter_map(|object_id| {
                let unstructured_ownership: Vec<UnstructuredOwnershipInfo> = schema
                    .ownership_unstructured(&object_id)
                    .iter()
                    .map(|v| v.into())
                    .collect();
                let ownership: Vec<StructuredOwnershipInfo> = schema
                    .rightholders(&object_id)
                    .iter()
                    .filter_map(|(rightholder, rights)| {
                        StructuredOwnershipInfo::from_rights(rights, rightholder).ok()
                    })
                    .collect();
                schema
                    .objects_identity()
                    .get(&object_id)
                    .map(|object| ObjectInformationDto {
                        object: object.into(),
                        data: schema.objects().get(&object_id).unwrap(),
                        ownership,
                        unstructured_ownership,
                    })
            })
            .collect()
    } else {
        iter.iter_rev()
            .take(limit)
            .filter_map(|object_id| {
                let unstructured_ownership: Vec<UnstructuredOwnershipInfo> = schema
                    .ownership_unstructured(&object_id)
                    .iter()
                    .map(|v| v.into())
                    .collect();
                let ownership: Vec<StructuredOwnershipInfo> = schema
                    .rightholders(&object_id)
                    .iter()
                    .filter_map(|(rightholder, rights)| {
                        StructuredOwnershipInfo::from_rights(rights, rightholder).ok()
                    })
                    .collect();
                schema
                    .objects_identity()
                    .get(&object_id)
                    .map(|object| ObjectInformationDto {
                        object: object.into(),
                        data: schema.objects().get(&object_id).unwrap(),
                        ownership,
                        unstructured_ownership,
                    })
            })
            .collect()
    };

    PaginationPage {
        data: tmp,
        limit,
        from: from.map(Into::into),
    }
}

pub fn get_contract_checks(
    state: State,
    requestor: MemberIdentity,
    contract_id: &ContractId,
) -> Result<HashMap<CheckKey, CheckInfo>> {
    let schema = Schema::new(state.snapshot());
    let contract = schema
        .contracts()
        .get(contract_id)
        .ok_or_else(|| Error::no_contract(contract_id))?;
    if !contract.is_member(&requestor) {
        Error::no_permissions().ok()?
    }
    schema
        .checks(contract_id)
        .into_iter()
        .map(|(k, v)| {
            CheckKey::try_from(k)
                .map_err(|_| Error::internal_bad_struct("CheckKey"))
                .map(|k| (k, CheckInfo::from(v)))
        })
        .collect()
}

pub fn get_lot_checks(state: State, lot_id: &LotId) -> Result<HashMap<CheckKey, CheckInfo>> {
    let schema = Schema::new(state.snapshot());
    if !schema.lots().contains(lot_id) {
        Error::no_lot(lot_id).ok()?
    };

    schema
        .checks(lot_id)
        .into_iter()
        .map(|(k, v)| {
            CheckKey::try_from(k)
                .map_err(|_| Error::internal_bad_struct("CheckKey"))
                .map(|k| (k, CheckInfo::from(v)))
        })
        .collect()
}

pub fn get_contract_status(state: State, contract_tx_hash: &ContractId) -> Result<ContractStatus> {
    let schema = Schema::new(state.snapshot());
    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    let status: ContractStatus = ContractStatus::try_from(contract.state())?;

    Ok(status)
}

pub fn get_contract_conditions(
    state: State,
    contract_tx_hash: &ContractId,
) -> Result<ConditionsInfo> {
    let schema = Schema::new(state.snapshot());
    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    ConditionsInfo::try_from(contract.conditions())
}

pub fn attach_contract_other_file(
    state: State,
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    attachment: Attachment,
) -> Result<TxHash> {
    attachment.verify()?;
    if attachment.metadata().file_type() != AttachmentType::Other as u8 {
        Error::bad_file_type("file type is not 'other'").ok()?;
    }
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    // share document with seller and buyer
    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::attach_contract_other_file(
        requestor,
        contract_tx_hash,
        attachment,
        share,
        cert,
    );
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn attach_contract_main_file(
    state: State,
    contract_tx_hash: &ContractId,
    attachment: Attachment,
) -> Result<TxHash> {
    attachment.verify()?;
    // Deprecated in https://aj.srvdev.ru/browse/FIPSOP-1045
    // if attachment.metadata().file_type() == AttachmentType::Other as u8 {
    //     Error::bad_file_type("file type must be one of: 'deed', 'application', 'notification'")
    //         .ok()?;
    // }
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    // share document with seller and buyer
    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::attach_contract_main_file(contract_tx_hash, attachment, share, cert);
    send_private(state, tx)
}

pub fn delete_contract_files(
    state: State,
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    doc_tx_hashes: &[DocumentId],
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::delete_contract_files(
        requestor,
        contract_tx_hash,
        doc_tx_hashes,
        share,
        cert,
    );
    send_private(state, tx)
}

pub fn confirm_contract(
    state: State,
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    deed_tx_hash: &DocumentId,
    application_tx_hash: &DocumentId,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::confirm_contract(
        requestor,
        contract_tx_hash,
        deed_tx_hash,
        application_tx_hash,
        share,
        cert,
    );
    send_private(state, tx)
}

pub fn refuse_contract(
    state: State,
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    reason: &str,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());

    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
    if !contract.is_member(&requestor) {
        return Err(Error::no_permissions());
    };

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::refuse_contract(requestor, contract_tx_hash, reason, share, cert);
    send_private(state, tx)
}

pub fn update_contract(
    state: State,
    contract_tx_hash: &ContractId,
    requestor: MemberIdentity,
    price: Cost,
    conditions: Conditions,
    contract_correspondence: Option<String>,
    objects_correspondence: Option<String>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    // TODO: Add requestor and rightholder nodes to share
    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::update_contract(
        contract_tx_hash,
        requestor,
        price,
        conditions,
        contract_correspondence,
        objects_correspondence,
        share,
        cert,
    );
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn approve_contract(
    state: State,
    contract_tx_hash: &ContractId,
    attachment: Option<Attachment>,
) -> Result<TxHash> {
    let attachment = attachment
        .map(|attach| {
            attach
                .metadata()
                .file_type()
                .try_into()
                .and_then(|file_metadata: AttachmentType| {
                    if file_metadata != AttachmentType::Notification {
                        Error::bad_file_type("file type have to be 'notification'").ok()
                    } else {
                        Ok(())
                    }
                })
                .and_then(|_| attach.verify())
                .map(|_| attach)
        })
        .transpose()?;
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();
    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::approve_contract(contract_tx_hash, attachment, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn reject_contract(
    state: State,
    contract_tx_hash: &ContractId,
    reason: &str,
    attachment: Option<Attachment>,
) -> Result<TxHash> {
    let attachment = attachment
        .map(|attach| {
            attach
                .metadata()
                .file_type()
                .try_into()
                .and_then(|file_metadata: AttachmentType| {
                    if file_metadata != AttachmentType::Notification {
                        Error::bad_file_type("file type have to be 'notification'").ok()
                    } else {
                        Ok(())
                    }
                })
                .and_then(|_| attach.verify())
                .map(|_| attach)
        })
        .transpose()?;
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();
    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::reject_contract(contract_tx_hash, reason, attachment, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn add_participant(state: State, user: MemberIdentity, node_name: &str) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::add_participant(user, node_name, cert);
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn register_contract(state: State, contract_tx_hash: &ContractId) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;
    let tx = transactions::register_contract(contract_tx_hash, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn await_user_action_contract(state: State, contract_tx_hash: &ContractId) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;
    let tx = transactions::await_user_action_contract(contract_tx_hash, share, cert);
    send_private(state, tx)
}

pub fn sign_contract(
    state: State,
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    deed_sign: Sign,
    application_sign: Sign,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;
    let tx = transactions::sign_contract(
        requestor,
        contract_tx_hash,
        deed_sign,
        application_sign,
        share,
        cert,
    );
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn contract_submit_checks(
    state: State,
    contract_tx_hash: &ContractId,
    checks: HashMap<CheckKey, CheckInfo>,
    is_undef: bool,
    reference_number: Option<String>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;
    let checks = checks
        .into_iter()
        .map(|(key, result)| Check::new(key as u16, result.into()))
        .collect();
    let tx = transactions::contract_submit_checks(
        contract_tx_hash,
        checks,
        is_undef,
        reference_number,
        share,
        cert,
    );
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn lot_submit_checks(
    state: State,
    lot_tx_hash: &LotId,
    checks: HashMap<CheckKey, CheckInfo>,
    is_undef: bool,
    reference_number: Option<String>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();
    if !schema.lots().contains(lot_tx_hash) {
        Error::no_lot(lot_tx_hash).ok()?
    }
    let checks = checks
        .into_iter()
        .map(|(key, result)| Check::new(key as u16, result.into()))
        .collect();
    let tx = transactions::lot_submit_checks(lot_tx_hash, checks, is_undef, reference_number, cert);
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn contract_reference_number(
    state: State,
    contract_tx_hash: &ContractId,
    reference_number: &str,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx =
        transactions::contract_reference_number(contract_tx_hash, reference_number, share, cert);
    send_private(state, tx)
}

#[inline]
pub fn get_contract_documents<T: AsRef<dyn Snapshot>>(
    schema: &Schema<T>,
    contract_tx_hash: &ContractId,
) -> Result<ContractDocuments> {
    let deed = schema
        .contract_deed(contract_tx_hash)
        .map(TryInto::try_into)
        .transpose()?;
    let application = schema
        .contract_application(contract_tx_hash)
        .map(TryInto::try_into)
        .transpose()?;
    let stored_docs = schema
        .contract_files(contract_tx_hash)
        .iter()
        .map(|(k, v)| {
            v.try_into()
                .map(|v| AttachmentMetadataWithHashDto::new(k, v))
        })
        .collect::<Result<Vec<AttachmentMetadataWithHashDto>>>()?;
    let notifications = schema
        .contract_notifications(contract_tx_hash)
        .iter()
        .map(|(k, v)| {
            v.try_into()
                .map(|v| AttachmentMetadataWithHashDto::new(k, v))
        })
        .collect::<Result<Vec<AttachmentMetadataWithHashDto>>>()?;
    Ok(ContractDocuments {
        deed_file: deed,
        application_file: application,
        other_files: stored_docs,
        notification_files: notifications,
    })
}

pub fn get_contract(state: State, contract_tx_hash: &ContractId) -> Result<ContractInfo> {
    let schema = Schema::new(state.snapshot());

    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    let reference_number = schema.contract_reference_number(contract_tx_hash);

    let calculations: Result<Vec<CalculationWithPaymentDetailInfo>> = schema
        .get_contract_payment_details(contract_tx_hash)
        .into_iter()
        .map(|v| v.try_into())
        .collect();

    let contacts = schema
        .correspondence_contacts()
        .get(contract_tx_hash)
        .unwrap_or_else(|| CorrespondenceContacts::new(None, None));

    let documents = self::get_contract_documents(&schema, contract_tx_hash)?;

    let contract_info = ContractInfo {
        buyer: contract.buyer().into(),
        seller: contract.seller().into(),
        price: contract.price(),
        conditions: contract.conditions().try_into()?,
        status: ContractStatus::try_from(contract.state())?.to_string(),
        documents,
        reference_number,
        calculations: calculations?,
        is_undefined: contract.is_undefined(),
        contract_correspondence: contacts.contract_correspondence(),
        objects_correspondence: contacts.objects_correspondence(),
    };
    Ok(contract_info)
}

#[cfg(feature = "internal_api")]
pub fn object_participates(state: State, object: ObjectIdentity) -> Result<ObjectParticipates> {
    let schema = Schema::new(state.snapshot());
    let object_id = object.id();
    let lots = schema
        .lots_to_invalidate(&object_id)
        .into_iter()
        .map(|val| val.0)
        .collect::<Vec<LotId>>();
    let contracts = schema
        .contracts_to_invalidate(&object_id)
        .into_iter()
        .map(|val| val.0)
        .collect::<Vec<ContractId>>();
    Ok(ObjectParticipates { lots, contracts })
}

pub fn tax_request(
    state: State,
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());

    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    if !contract.is_member(&requestor) {
        Error::no_permissions().ok()?
    }

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::tax_request(requestor, contract_tx_hash, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn add_tax_contract_calculation(
    state: State,
    contract_tx_hash: &ContractId,
    calculations: Vec<Calculation>,
    reference_number: Option<String>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::tax_contract_calculation(
        contract_tx_hash,
        calculations,
        reference_number,
        share,
        cert,
    );
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn add_tax_lot_calculation(
    state: State,
    lot_tx_hash: &LotId,
    calculations: Vec<Calculation>,
    reference_number: Option<String>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    schema
        .lots()
        .get(lot_tx_hash)
        .ok_or_else(|| Error::no_lot(lot_tx_hash))?;

    let cert = state.blockchain().certificate();

    let tx = transactions::tax_lot_calculation(lot_tx_hash, calculations, reference_number, cert);
    send(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn add_tax_with_payment_details(
    state: State,
    contract_tx_hash: &ContractId,
    payment_details: Vec<PaymentDetail>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::tax_with_payment_details(contract_tx_hash, payment_details, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn add_tax_status(
    state: State,
    contract_tx_hash: &ContractId,
    payment_id: &str,
    status: PaymentStatus,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::tax_status(contract_tx_hash, payment_id, status, share, cert);
    send_private(state, tx)
}

pub fn put_member_token(
    state: State,
    member: MemberIdentity,
    token: &str,
    oid: &str,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::member_token(member, token, oid, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn get_member_token(state: State, member: &MemberIdentity) -> Result<MemberEsiaTokenDto> {
    let schema = Schema::new(state.snapshot());
    if !member.is_valid() {
        Error::bad_member_format(&member.to_string()).ok()?
    }
    schema
        .member_token(member)
        .ok_or_else(|| Error::no_member_token())
        .map(|v| v.into())
}

pub fn get_confirm_create_status(
    state: State,
    contract_id: &ContractId,
) -> Result<RequestConfirmDto> {
    let schema = Schema::new(state.snapshot());
    let contract_status: ContractStatus = schema
        .contracts()
        .get(contract_id)
        .ok_or_else(|| Error::no_contract(contract_id))?
        .state()
        .try_into()?;
    let res = match contract_status {
        ContractStatus::RequestConfirm(c) => RequestConfirmDto {
            status: c.into(),
            status_gone: false,
        },
        _ => RequestConfirmDto {
            status: ConfirmDto {
                buyer: true,
                seller: true,
            },
            status_gone: true,
        },
    };
    Ok(res)
}

pub fn post_confirm_create(
    state: State,
    requestor_id: MemberIdentity,
    contract_tx_hash: &ContractId,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
    if !contract.is_member(&requestor_id) {
        Error::no_permissions().ok()?
    }

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::contract_confirm_create(requestor_id, contract_tx_hash, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn post_unconfirm_create(
    state: State,
    member_id: MemberIdentity,
    contract_tx_hash: &ContractId,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
    if !contract.is_member(&member_id) {
        Error::no_permissions().ok()?
    }

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::contract_unconfirm_create(member_id, contract_tx_hash, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn post_contract_new(state: State, contract_tx_hash: &ContractId) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    contract.apply(Action::New)?;

    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::contract_new(contract_tx_hash, share, cert);
    send_private(state, tx)
}

fn send(state: State, tx: Box<dyn Transaction>) -> Result<TxHash> {
    trace!("SEND TRANS: {:?}", tx);
    let tx_hash = TxHash::from(tx.as_ref());
    state
        .sender()
        .send(tx, &state.blockchain(), None)
        .map(move |_| tx_hash)
        .map_err(|e| Error::unable_to_send_msg(&e.to_string()))
}

fn send_private(state: State, tx: Box<dyn Transaction>) -> Result<TxHash> {
    trace!("SEND PRIVATE TRANS: {:?}", tx);
    state
        .sender()
        .send_private(tx, &state.blockchain())
        .map(TxHash::from)
        .map_err(|e| Error::unable_to_send_msg(&e.to_string()))
}
