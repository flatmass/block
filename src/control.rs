use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use chrono::{DateTime, Utc};

use blockp_core::api::ServiceApiState as State;
use blockp_core::blockchain::Transaction;
use blockp_core::crypto::{Hash, PublicKey};
use blockp_core::node::{TransactionSend, TransactionSendPrivate};

#[cfg(feature = "internal_api")]
use crate::data::attachment::SignedAttachment;
use crate::data::attachment::{Attachment, AttachmentType, DocumentId, Sign};
#[cfg(feature = "internal_api")]
use crate::data::conditions::Check;
use crate::data::conditions::{CheckKey, Conditions};
use crate::data::contract::{ContractId, ContractStatus};
use crate::data::cost::Cost;
use crate::data::lot::{LotId, LotStatus};
use crate::data::member::MemberIdentity;
use crate::data::object::ObjectIdentity;
#[cfg(feature = "internal_api")]
use crate::data::ownership::{Ownership, OwnershipUnstructured};
use crate::dto::*;
use crate::error::{Error, Result};
use crate::schema::Schema;
use crate::transactions::{self, get_private_tx, get_transaction, OwnershipTransactions};
use crate::util::dedup_naive;

pub fn add_object_request(
    state: State,
    owner: MemberIdentity,
    object: ObjectIdentity,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::add_object_request(owner, object, cert);
    send(state, tx)
}

pub fn add_object_group_request(state: State, owner: MemberIdentity) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::add_object_group_request(owner, cert);
    send(state, tx)
}

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
    owner: MemberIdentity,
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<OwnershipInfo>,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let (structured_ownership, unstructured_ownership) = split_ownership(ownership)?;
    let tx = transactions::add_object(
        owner,
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
    owner: MemberIdentity,
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<OwnershipInfo>,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let (structured_ownership, unstructured_ownership) = split_ownership(ownership)?;
    let tx = transactions::update_object(
        owner,
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

pub fn attach_file(
    state: State,
    requestor: MemberIdentity,
    name: &str,
    blob: &[u8],
    file_type: AttachmentType,
    members: Vec<MemberIdentity>,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let file = Attachment::new(name, blob, file_type as u8);
    let schema = Schema::new(state.snapshot());
    let members = dedup_naive(members);

    let share = members
        .iter()
        .chain(std::iter::once(&requestor))
        .flat_map(|p| {
            schema
                .participants(&p.id())
                .into_iter()
                .collect::<Vec<String>>()
        })
        .map(|s| PublicKey::from_slice(s.as_bytes()).ok_or(Error::bad_stored_member(s.as_str())))
        .collect::<Result<Vec<PublicKey>>>()?;

    let share = dedup_naive(share);

    let tx = transactions::attach_file(requestor, file, cert, members, share);
    send_private(state, tx)
}

pub fn delete_files(
    state: State,
    requestor: MemberIdentity,
    doc_tx_hashes: &[DocumentId],
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let tx = transactions::delete_files(requestor, doc_tx_hashes, cert);
    send_private(state, tx)
}

pub fn get_file(state: State, requestor: MemberIdentity, doc_tx_hash: &Hash) -> Result<Attachment> {
    let schema = Schema::new(state.snapshot());
    let uid = &requestor.id();
    if !schema.attachments(uid).contains(doc_tx_hash) {
        return Error::no_permissions().ok();
    }
    let txset = get_private_tx(&schema, doc_tx_hash)?;
    match txset {
        OwnershipTransactions::AttachFile(doc_tx) => Ok(doc_tx.file()),
        _ => Error::unexpected_tx_type(doc_tx_hash).ok(),
    }
}

pub fn add_attachment_sign(
    state: State,
    requestor_id: MemberIdentity,
    doc_tx_hash: &Hash,
    sign: Sign,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let schema = Schema::new(state.snapshot());

    let txset = get_private_tx(&schema, doc_tx_hash)?;
    let share = match txset {
        OwnershipTransactions::AttachFile(doc_tx) => Ok(doc_tx.share()),
        _ => Error::unexpected_tx_type(doc_tx_hash).ok(),
    }?;

    let tx = transactions::add_attachment_sign(requestor_id, doc_tx_hash, sign, cert, share);
    send_private(state, tx)
}

pub fn open_lot(
    state: State,
    requestor: MemberIdentity,
    info: LotInfo,
    conditions: Conditions,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let lot = info.into_lot()?;
    lot.verify()?;
    let tx = transactions::open_lot(requestor, lot, conditions, cert);
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
pub fn draft_contract(
    state: State,
    contract_tx_hash: &ContractId,
    doc_tx_hashes: &[DocumentId],
    deed_tx_hash: &DocumentId,
    application_tx_hash: &DocumentId,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();
    let share = schema.get_contract_share(contract_tx_hash)?;
    let tx = transactions::draft_contract(
        contract_tx_hash,
        doc_tx_hashes,
        deed_tx_hash,
        application_tx_hash,
        share,
        cert,
    );
    send_private(state, tx)
}

pub fn acquire_lot(state: State, requestor: MemberIdentity, lot_id: &LotId) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let lot_tx = match get_transaction(&schema, lot_id) {
        Ok(OwnershipTransactions::OpenLot(tx)) => Ok(tx),
        _ => Error::no_transaction(lot_id).ok(),
    }?;
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
    rightholder: MemberIdentity,
    price: Cost,
    conditions: Conditions,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let share = [&rightholder, &requestor]
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
    let tx = transactions::purchase_offer(requestor, rightholder, price, conditions, share, cert);
    send_private(state, tx)
}

pub fn add_tax(
    state: State,
    contract_tx_hash: ContractId,
    requestor: MemberIdentity,
    number: String,
    payment_date: DateTime<Utc>,
    amount: Cost,
) -> Result<TxHash> {
    let cert = state.blockchain().certificate();
    let schema = Schema::new(state.snapshot());
    let share = schema.get_contract_share(&contract_tx_hash)?;
    let tx = transactions::add_tax(
        contract_tx_hash,
        requestor,
        number,
        payment_date,
        amount,
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

pub fn get_lot_info(state: State, lot_id: &LotId) -> Result<LotInfo> {
    let schema = Schema::new(state.snapshot());
    let state = schema
        .lot_states()
        .get(lot_id)
        .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;
    let price = Cost::from(state.price());
    let status_val = state.status();
    let status = LotStatus::try_from(status_val)
        .map_err(|_| Error::bad_lot_status(&status_val.to_string()))?;

    schema
        .lots()
        .get(&lot_id)
        .ok_or_else(|| Error::no_lot(&lot_id))
        .map(|lot| LotInfo::from(&lot).set_price(price).set_status(status))
}

pub fn get_lot_info_with_objects(state: State, lot_id: &LotId) -> Result<LotInfoWithObjects> {
    let schema = Schema::new(state.snapshot());
    let lot_info = get_lot_info(state, lot_id)?;
    let lot_conditions = schema
        .lot_conditions()
        .get(lot_id)
        .ok_or_else(|| Error::no_lot(lot_id))?;
    let lot_info_with_objects = LotInfoWithObjects {
        lot: lot_info,
        objects: lot_conditions
            .objects()
            .into_iter()
            .map(|v| v.object().into())
            .collect(),
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

pub fn get_all_lots(state: State) -> Result<Lots> {
    let schema = Schema::new(state.snapshot());
    Ok(schema.lots().keys().collect())
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

pub fn get_member_objects(state: State, member: MemberIdentity) -> Result<Vec<ObjectIdentity>> {
    let schema = Schema::new(state.snapshot());
    Ok(schema
        .ownership(&member.id())
        .into_iter()
        .map(|(_hash, object)| object)
        .collect())
}

pub fn get_object(state: State, object: ObjectIdentity) -> Result<String> {
    let schema = Schema::new(state.snapshot());
    schema
        .objects()
        .get(&object.id())
        .ok_or_else(|| Error::no_object(&object))
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

pub fn attach_contract_file(
    state: State,
    requestor: MemberIdentity,
    name: &str,
    contract_tx_hash: &ContractId,
    blob: &[u8],
    file_type: AttachmentType,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();
    let attachment = Attachment::new(name, blob, file_type as u8);

    // share document with seller and buyer
    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx =
        transactions::attach_contract_file(requestor, contract_tx_hash, attachment, share, cert);
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
    doc_tx_hashes: &[DocumentId],
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx = transactions::confirm_contract(
        requestor,
        contract_tx_hash,
        deed_tx_hash,
        application_tx_hash,
        doc_tx_hashes,
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
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    // TODO: Add requestor and rightholder nodes to share
    let share = schema.get_contract_share(contract_tx_hash)?;

    let tx =
        transactions::update_contract(contract_tx_hash, requestor, price, conditions, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn reject_contract(
    state: State,
    contract_tx_hash: &ContractId,
    reason: &str,
    name: Option<&str>,
    blob: Option<&[u8]>,
    sign: Option<Sign>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;
    let attachment = name
        .zip(blob)
        .map(|(name, blob)| Attachment::new(name, blob, AttachmentType::Other as u8));
    let signed_file = attachment
        .zip(sign)
        .map(|(attachment, sign)| SignedAttachment::new(attachment, sign));
    let tx = transactions::reject_contract(contract_tx_hash, reason, signed_file, share, cert);
    send_private(state, tx)
}

#[cfg(feature = "internal_api")]
pub fn approve_contract(
    state: State,
    contract_tx_hash: &ContractId,
    name: &str,
    blob: &[u8],
    sign: Sign,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();
    let share = schema.get_contract_share(contract_tx_hash)?;
    let attachment = Attachment::new(name, blob, AttachmentType::Other as u8);
    let signed_file = SignedAttachment::new(attachment, sign);
    signed_file.verify()?;
    let tx = transactions::approve_contract(contract_tx_hash, signed_file, share, cert);
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
pub fn submit_checks(
    state: State,
    contract_tx_hash: &ContractId,
    checks: HashMap<CheckKey, CheckInfo>,
) -> Result<TxHash> {
    let schema = Schema::new(state.snapshot());
    let cert = state.blockchain().certificate();

    let share = schema.get_contract_share(contract_tx_hash)?;
    let checks = checks
        .into_iter()
        .map(|(key, result)| Check::new(key as u16, result.into()))
        .collect();
    let tx = transactions::submit_checks(contract_tx_hash, checks, share, cert);
    send_private(state, tx)
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

pub fn get_contract(state: State, contract_tx_hash: &ContractId) -> Result<ContractInfo> {
    let schema = Schema::new(state.snapshot());

    let contract = schema
        .contracts()
        .get(contract_tx_hash)
        .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

    let deed = schema.contract_deed(contract_tx_hash);
    let application = schema.contract_application(contract_tx_hash);
    let stored_docs: Vec<DocumentId> = schema.contract_files(contract_tx_hash).keys().collect();
    let reference_number = schema.contract_reference_number(contract_tx_hash);
    let contract_info = ContractInfo {
        buyer: MemberInfo(contract.buyer()),
        seller: MemberInfo(contract.seller()),
        price: contract.price(),
        conditions: contract.conditions().try_into()?,
        status: ContractStatus::try_from(contract.state())?.to_string(),
        deed_tx_hash: deed,
        application_tx_hash: application,
        stored_docs,
        reference_number,
    };
    Ok(contract_info)
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
