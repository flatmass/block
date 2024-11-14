use std::collections::HashMap;
use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};

use chrono::{DateTime, Utc};
use rand::{thread_rng, RngCore};

use blockp_core::blockchain::{
    Blockchain, ExecutionError, ExecutionResult, Transaction, TransactionSet,
};
use blockp_core::crypto::{self, get_cert_from_detached_sign, Certificate, Hash, PublicKey};
use blockp_core::messages::RawMessage;
use blockp_core::storage::{Fork, Snapshot};

use crate::data::attachment::{Attachment, AttachmentType, DocumentId, Sign, SignedAttachment};
use crate::data::conditions::{Check, CheckKey, Conditions};
use crate::data::contract::{Action, Contract, ContractId, ContractStatus, Tax};
use crate::data::cost::Cost;
use crate::data::lot::{Bid, Lot, LotId, LotState, LotStatus};
use crate::data::member::{MemberId, MemberIdentity};
use crate::data::object::ObjectIdentity;
use crate::data::ownership::{Ownership, OwnershipUnstructured, Rights};
use crate::data::request::Request;
use crate::data::strings::verify_node_name;
use crate::error::{self, Error};
use crate::schema::Schema;
use crate::util::contains_diplicates;

impl From<Error> for ExecutionError {
    fn from(err: Error) -> Self {
        ExecutionError::with_description(err.code(), err.info())
    }
}

pub fn add_object_request(
    requestor: MemberIdentity,
    object: ObjectIdentity,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AddObjectRequest::new(
        salt(),
        TxType::AddObjectRequest as u8,
        requestor,
        object,
        cert,
    )
    .into()
}

pub fn add_object_group_request(
    requestor: MemberIdentity,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AddObjectGroupRequest::new(salt(), TxType::AddObjectGroupRequest as u8, requestor, cert).into()
}

#[cfg(feature = "internal_api")]
pub fn add_object(
    owner: MemberIdentity,
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<Ownership>,
    ownership_unstructured: Vec<OwnershipUnstructured>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AddObject::new(
        salt(),
        TxType::AddObject as u8,
        owner,
        object,
        data,
        ownership,
        ownership_unstructured,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn update_object(
    owner: MemberIdentity,
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<Ownership>,
    ownership_unstructured: Vec<OwnershipUnstructured>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    UpdateObject::new(
        salt(),
        TxType::UpdateObject as u8,
        owner,
        object,
        data,
        ownership,
        ownership_unstructured,
        cert,
    )
    .into()
}

pub fn attach_file(
    requestor: MemberIdentity,
    file: Attachment,
    cert: &Certificate,
    members: Vec<MemberIdentity>,
    share: Vec<PublicKey>,
) -> Box<dyn Transaction> {
    AttachFile::new(
        salt(),
        TxType::AttachFile as u8,
        requestor,
        file,
        members,
        share,
        cert,
    )
    .into()
}

pub fn delete_files(
    requestor: MemberIdentity,
    doc_hashes: &[DocumentId],
    cert: &Certificate,
) -> Box<dyn Transaction> {
    DeleteFiles::new(
        salt(),
        TxType::DeleteFiles as u8,
        requestor,
        doc_hashes,
        cert,
    )
    .into()
}

pub fn add_attachment_sign(
    requestor: MemberIdentity,
    doc_tx_hash: &Hash,
    sign: Sign,
    cert: &Certificate,
    share: Vec<PublicKey>,
) -> Box<dyn Transaction> {
    AddAttachmentSign::new(
        salt(),
        TxType::AddAttachmentSign as u8,
        requestor,
        doc_tx_hash,
        sign,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn add_participant(
    member: MemberIdentity,
    node_name: &str,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AddParticipant::new(0, TxType::AddParticipant as u8, member, node_name, cert).into()
}

pub fn open_lot(
    requestor: MemberIdentity,
    lot: Lot,
    conditions: Conditions,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    OpenLot::new(
        salt(),
        TxType::OpenLot as u8,
        requestor,
        lot,
        conditions,
        cert,
    )
    .into()
}

pub fn close_lot(
    requestor: MemberIdentity,
    lot_id: &LotId,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    CloseLot::new(0, TxType::CloseLot as u8, requestor, lot_id, cert).into()
}

#[cfg(feature = "internal_api")]
pub fn edit_lot_status(
    lot_id: &LotId,
    cert: &Certificate,
    status: LotStatus,
) -> Box<dyn Transaction> {
    EditLotStatus::new(
        salt(),
        TxType::EditLotStatus as u8,
        lot_id,
        status as u8,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn execute_lot(lot_tx_hash: &LotId, cert: &Certificate) -> Box<dyn Transaction> {
    ExecuteLot::new(0, TxType::ExecuteLot as u8, lot_tx_hash, cert).into()
}

pub fn extend_lot_period(
    requestor: MemberIdentity,
    lot_tx_hash: &LotId,
    new_expiration_date: DateTime<Utc>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ExtendLotPeriod::new(
        0,
        TxType::ExtendLotPeriod as u8,
        requestor,
        lot_tx_hash,
        new_expiration_date,
        cert,
    )
    .into()
}

pub fn acquire_lot(
    requestor: MemberIdentity,
    lot_tx_hash: &LotId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AcquireLot::new(
        salt(),
        TxType::AcquireLot as u8,
        requestor,
        lot_tx_hash,
        share,
        cert,
    )
    .into()
}

pub fn purchase_offer(
    requestor: MemberIdentity,
    rightholder: MemberIdentity,
    price: Cost,
    conditions: Conditions,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    PurchaseOffer::new(
        salt(),
        TxType::PurchaseOffer as u8,
        requestor,
        rightholder,
        price.into(),
        conditions,
        share,
        cert,
    )
    .into()
}

pub fn add_tax(
    contract_tx_hash: ContractId,
    requestor: MemberIdentity,
    number: String,
    payment_date: DateTime<Utc>,
    amount: Cost,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AddTaxInfo::new(
        0,
        TxType::AddTaxInfo as u8,
        &contract_tx_hash,
        requestor,
        number.as_str(),
        payment_date,
        amount.into(),
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn draft_contract(
    contract_tx_hash: &ContractId,
    doc_tx_hashes: &[DocumentId],
    deed_tx_hash: &DocumentId,
    application_tx_hash: &DocumentId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    DraftContract::new(
        0,
        TxType::DraftContract as u8,
        contract_tx_hash,
        doc_tx_hashes,
        deed_tx_hash,
        application_tx_hash,
        share,
        cert,
    )
    .into()
}

pub fn refuse_contract(
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    reason: &str,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    RefuseContract::new(
        0,
        TxType::RefuseContract as u8,
        requestor,
        contract_tx_hash,
        reason,
        share,
        cert,
    )
    .into()
}

pub fn confirm_contract(
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    deed_tx_hash: &DocumentId,
    application_tx_hash: &DocumentId,
    doc_tx_hashes: &[DocumentId],
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ConfirmContract::new(
        salt(),
        TxType::ConfirmContract as u8,
        requestor,
        contract_tx_hash,
        deed_tx_hash,
        application_tx_hash,
        doc_tx_hashes,
        share,
        cert,
    )
    .into()
}

pub fn update_contract(
    contract_tx_hash: &ContractId,
    requestor: MemberIdentity,
    price: Cost,
    conditions: Conditions,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    UpdateContract::new(
        salt(),
        TxType::UpdateContract as u8,
        contract_tx_hash,
        requestor,
        price.into(),
        conditions,
        share,
        cert,
    )
    .into()
}

pub fn attach_contract_file(
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    file: Attachment,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AttachContractFile::new(
        salt(),
        TxType::AttachContractFile as u8,
        requestor,
        contract_tx_hash,
        file,
        share,
        cert,
    )
    .into()
}

pub fn delete_contract_files(
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    doc_tx_hashes: &[DocumentId],
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    DeleteContractFiles::new(
        salt(),
        TxType::DeleteContractFiles as u8,
        requestor,
        contract_tx_hash,
        doc_tx_hashes,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn approve_contract(
    contract_tx_hash: &ContractId,
    signed_file: SignedAttachment,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ApproveContract::new(
        0,
        TxType::ApproveContract as u8,
        contract_tx_hash,
        signed_file,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn reject_contract(
    contract_tx_hash: &ContractId,
    reason: &str,
    signed_file: Option<SignedAttachment>,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    RejectContract::new(
        0,
        TxType::RejectContract as u8,
        contract_tx_hash,
        reason,
        signed_file,
        share,
        cert,
    )
    .into()
}

pub fn sign_contract(
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    deed_sign: Sign,
    application_sign: Sign,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    SignContract::new(
        salt(),
        TxType::SignContract as u8,
        requestor,
        contract_tx_hash,
        deed_sign,
        application_sign,
        share,
        cert,
    )
    .into()
}

pub fn add_bid(
    requestor: MemberIdentity,
    lot_id: &LotId,
    bid: Bid,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AddBid::new(0, TxType::AddBid as u8, requestor, lot_id, bid, cert).into()
}

#[cfg(feature = "internal_api")]
pub fn publish_bids(lot_id: &LotId, bids: Vec<u64>, cert: &Certificate) -> Box<dyn Transaction> {
    PublishBids::new(0, TxType::PublishBids as u8, lot_id, bids, cert).into()
}

#[cfg(feature = "internal_api")]
pub fn register_contract(
    contract_tx_hash: &ContractId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    RegisterContract::new(
        0,
        TxType::RegisterContract as u8,
        contract_tx_hash,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn await_user_action_contract(
    contract_tx_hash: &ContractId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AwaitUserActionContract::new(
        0,
        TxType::AwaitUserActionContract as u8,
        contract_tx_hash,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn submit_checks(
    contract_tx_hash: &ContractId,
    checks: Vec<Check>,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    SubmitChecks::new(
        0,
        TxType::SubmitChecks as u8,
        contract_tx_hash,
        checks,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn contract_reference_number(
    contract_tx_hash: &ContractId,
    reference_number: &str,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ContractReferenceNumber::new(
        0,
        TxType::ContractReferenceNumber as u8,
        contract_tx_hash,
        reference_number,
        share,
        cert,
    )
    .into()
}

fn convert_tx<T: AsRef<dyn Snapshot>>(
    tx_hash: &Hash,
    raw: RawMessage,
) -> error::Result<OwnershipTransactions> {
    OwnershipTransactions::tx_from_raw(raw).map_err(|_| Error::unexpected_tx_type(tx_hash))
}

pub fn get_transaction<T: AsRef<dyn Snapshot>>(
    schema: &Schema<T>,
    tx_hash: &Hash,
) -> error::Result<OwnershipTransactions> {
    let raw = schema
        .core_transactions()
        .get(tx_hash)
        .ok_or(Error::no_transaction(tx_hash))?;
    convert_tx::<T>(tx_hash, raw)
}

pub fn get_private_tx<T: AsRef<dyn Snapshot>>(
    schema: &Schema<T>,
    pub_tx_hash: &Hash,
) -> error::Result<OwnershipTransactions> {
    let pub_tx_raw = schema
        .core_transactions()
        .get(pub_tx_hash)
        .ok_or(Error::no_transaction(pub_tx_hash))?;

    let priv_tx_raw = Blockchain::from_private_tx(&pub_tx_raw, &schema.core_private_data())
        .ok_or(Error::no_private_data(pub_tx_hash))?;

    convert_tx::<T>(pub_tx_hash, priv_tx_raw)
}

fn _member_matches_sign(member: &MemberIdentity, sign: &Sign) -> Result<(), Error> {
    const OGRN_OID: &'static str = "1.2.643.100.1";
    const OGRNIP_OID: &'static str = "1.2.643.100.5";
    const SNILS_OID: &'static str = "1.2.643.100.3";

    let certificate = get_cert_from_detached_sign(sign.data())
        .map_err(|_| Error::bad_signature("unable to decode"))?
        .ok_or_else(|| Error::bad_signature("unable to extract certificate"))?;

    let certificate_member = match member.class() {
        0 => certificate
            .get_oid(OGRN_OID)
            .expect("Certificate::get_oid argument contains internal NULL byte")
            .ok_or_else(|| {
                Error::bad_signature(&format!(
                    "unable to extract member identifier, OID '{}' must be present",
                    OGRN_OID
                ))
            }),
        1 => certificate
            .get_oid(OGRNIP_OID)
            .expect("Certificate::get_oid argument contains internal NULL byte")
            .ok_or_else(|| {
                Error::bad_signature(&format!(
                    "unable to extract member identifier, OID '{}' must be present",
                    OGRNIP_OID
                ))
            }),
        2 => certificate
            .get_oid(SNILS_OID)
            .expect("Certificate::get_oid argument contains internal NULL byte")
            .ok_or_else(|| {
                Error::bad_signature(&format!(
                    "unable to extract member identifier, OID '{}' must be present",
                    SNILS_OID
                ))
            }),
        _ => panic!("`transactions::member_matches_sign` called with invalid member"),
    }?;

    if certificate_member == member.number() {
        Ok(())
    } else {
        Err(Error::bad_signature("signature does not match member"))
    }
}

/*fn get_time<T: AsRef<dyn Snapshot>>(schema: &Schema<T>) -> error::Result<DateTime<Utc><Utc>> {
    schema.time().get().ok_or(NoTimeProviderError.into())
}*/

fn salt() -> u64 {
    thread_rng().next_u64()
}

// TODO it's a temporary implementation for displaying transaction type
// needs to replace this solution with another one
// for example, inserting additional tx-typename field when serializing to json
#[repr(u8)]
#[allow(unused)]
enum TxType {
    AddObjectRequest = 1,
    AddObjectGroupRequest = 2,
    AddObject = 3,
    UpdateObject = 4,
    AttachFile = 5,
    DeleteFiles = 6,
    AddAttachmentSign = 7,
    AddParticipant = 11,
    OpenLot = 12,
    CloseLot = 13,
    EditLotStatus = 14,
    ExecuteLot = 15,
    AddBid = 16,
    PublishBids = 17,
    AcquireLot = 18,
    PurchaseOffer = 19,
    DraftContract = 20,
    RefuseContract = 21,
    ConfirmContract = 22,
    AttachContractFile = 23,
    DeleteContractFiles = 24,
    ApproveContract = 25,
    RejectContract = 26,
    UpdateContract = 27,
    RegisterContract = 28,
    AwaitUserActionContract = 29,
    SignContract = 30,
    SubmitChecks = 31,
    AddTaxInfo = 32,
    ContractReferenceNumber = 33,
    ExtendLotPeriod = 34,
}

transactions! {
    pub OwnershipTransactions {
        const SERVICE_ID = crate::service::SERVICE_ID;

        struct AddObjectRequest {
            _type: u8,
            requestor: MemberIdentity,
            object: ObjectIdentity,
        }

        struct AddObjectGroupRequest {
            _type: u8,
            requestor: MemberIdentity,
        }

        struct AddObject {
            _type: u8,
            owner: MemberIdentity,
            object: ObjectIdentity,
            data: &str,
            ownership: Vec<Ownership>,
            unstructured_ownership: Vec<OwnershipUnstructured>,
        }

        struct UpdateObject {
            _type: u8,
            owner: MemberIdentity,
            object: ObjectIdentity,
            data: &str,
            ownership: Vec<Ownership>,
            unstructured_ownership: Vec<OwnershipUnstructured>,
        }

        struct AttachFile {
            _type: u8,
            requestor: MemberIdentity,
            file: Attachment,
            members: Vec<MemberIdentity>,
            share: Vec<PublicKey>,
        }

        struct DeleteFiles {
            _type: u8,
            requestor: MemberIdentity,
            doc_tx_hashes: &[DocumentId],
        }

        struct AddAttachmentSign {
            _type: u8,
            requestor: MemberIdentity,
            doc_tx_hash: &DocumentId,
            sign: Sign,
            share: Vec<PublicKey>,
        }

        struct AddParticipant {
            _type: u8,
            member: MemberIdentity,
            node_name: &str,
        }

        struct OpenLot {
            _type: u8,
            requestor: MemberIdentity,
            lot: Lot,
            conditions: Conditions,
        }

        struct CloseLot {
            _type: u8,
            requestor: MemberIdentity,
            lot_tx_hash: &LotId,
        }

        struct EditLotStatus {
            _type: u8,
            lot_tx_hash: &LotId,
            status: u8,
        }

        struct ExecuteLot {
            _type: u8,
            lot_tx_hash: &LotId,
        }

        struct ExtendLotPeriod {
            _type: u8,
            requestor: MemberIdentity,
            lot_tx_hash: &LotId,
            new_expiration_date: DateTime<Utc>,
        }

        struct AddBid {
            _type: u8,
            requestor: MemberIdentity,
            lot_tx_hash: &LotId,
            bid: Bid,
        }

        struct PublishBids {
            _type: u8,
            lot_tx_hash: &LotId,
            bids: Vec<u64>,
        }

        struct AcquireLot {
            _type: u8,
            requestor: MemberIdentity,
            lot_tx_hash: &LotId,
            share: Vec<PublicKey>,
        }

        struct PurchaseOffer {
            _type: u8,
            requestor: MemberIdentity,
            rightholder: MemberIdentity,
            price: u64,
            conditions: Conditions,
            share: Vec<PublicKey>,
        }

        struct DraftContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            doc_tx_hashes: &[DocumentId],
            deed_tx_hash: &DocumentId,
            application_tx_hash: &DocumentId,
            share: Vec<PublicKey>,
        }

        struct RefuseContract {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            reason: &str,
            share: Vec<PublicKey>,
        }

        struct ConfirmContract {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            deed_tx_hash: &DocumentId,
            application_tx_hash: &DocumentId,
            doc_tx_hashes: &[DocumentId],
            share: Vec<PublicKey>,
        }

        struct AttachContractFile {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            file: Attachment,
            share: Vec<PublicKey>,
        }

        struct DeleteContractFiles {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            doc_tx_hashes: &[DocumentId],
            share: Vec<PublicKey>,
        }

        struct ApproveContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            signed_file: SignedAttachment,
            share: Vec<PublicKey>,
        }

        struct RejectContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            reason: &str,
            signed_file: Option<SignedAttachment>,
            share: Vec<PublicKey>,
        }

        struct UpdateContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            requestor: MemberIdentity,
            price: u64,
            conditions: Conditions,
            share: Vec<PublicKey>,
        }

        struct RegisterContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            share: Vec<PublicKey>,
        }

        struct AwaitUserActionContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            share: Vec<PublicKey>,
        }

        struct SignContract {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            deed_sign: Sign,
            application_sign: Sign,
            share: Vec<PublicKey>,
        }

        struct SubmitChecks {
            _type: u8,
            contract_tx_hash: &ContractId,
            checks: Vec<Check>,
            share: Vec<PublicKey>,
        }

        struct AddTaxInfo {
            _type: u8,
            contract_tx_hash: &ContractId,
            requestor: MemberIdentity,
            number: &str,
            payment_date: DateTime<Utc>,
            amount: u64,
            share: Vec<PublicKey>,
        }

        struct ContractReferenceNumber {
            _type: u8,
            contract_tx_hash: &ContractId,
            reference_number: &str,
            share: Vec<PublicKey>,
        }
    }
}

impl Transaction for AddAttachmentSign {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.requestor().is_valid() //&& self.sign().verify().is_ok()
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let doc_tx_hash = self.doc_tx_hash();
        let uid = &self.requestor().id();

        if !schema.attachments(uid).contains(doc_tx_hash) {
            Error::no_attachment(doc_tx_hash).ok()?
        }

        // let attachment: Attachment = {
        //     let doc_tx = get_private_tx(&schema, doc_tx_hash)?;
        //     match doc_tx {
        //         OwnershipTransactions::AttachFile(tx) if tx.requestor().id() == *uid => {
        //             Ok(tx.file())
        //         }
        //         OwnershipTransactions::AttachContractFile(tx) => Ok(tx.file()),
        //         _ => Error::unexpected_tx_type(doc_tx_hash).ok(),
        //     }
        // }?;
        // let doc_data = attachment.data();
        let sign = self.sign();
        // member_matches_sign(&self.requestor(), &sign)?;
        // sign.verify_data(doc_data)?;

        Ok(schema.add_attachment_sign(doc_tx_hash, uid, sign))
    }
}

impl Transaction for AttachFile {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.requestor().is_valid()
            && self.file().verify().is_ok()
            && !contains_diplicates(self.members())
            && !contains_diplicates(self.share())
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let data_hash = crypto::hash(self.file().data());
        schema.attach_file(&self.requestor().id(), tx_hash, data_hash);

        for p in self.members().iter() {
            schema.attach_file(&p.id(), tx_hash, data_hash);
        }
        Ok(())
    }
}

impl Transaction for DeleteFiles {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.requestor().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let uid = &self.requestor().id();

        let docs = self.doc_tx_hashes();

        for doc_tx_hash in docs {
            if !schema.attachments(uid).contains(doc_tx_hash) {
                Error::no_attachment(doc_tx_hash).ok()?
            }
            let doc_tx = get_private_tx(&schema, doc_tx_hash)?;
            match doc_tx {
                OwnershipTransactions::AttachFile(tx) if tx.requestor().id() == *uid => Ok(()),
                _ => Error::unexpected_tx_type(tx_hash).ok(),
            }?;

            schema.remove_attachment_sign(&uid, doc_tx_hash);
            schema.remove_file(&uid, doc_tx_hash);
        }

        Ok(())
    }
}

impl Transaction for AddObjectRequest {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.requestor().is_valid() && self.object().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        schema.put_request(&self.requestor().id(), Request::add_object(tx_hash));
        Ok(())
    }
}

impl Transaction for AddObjectGroupRequest {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.requestor().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        schema.put_request(&self.requestor().id(), Request::add_object_group(tx_hash));
        Ok(())
    }
}

impl Transaction for AddObject {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.owner().is_valid() && self.object().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let object = self.object();
        let obj_id = &object.id();
        if schema.objects().contains(obj_id) {
            Error::duplicate_object(&object).ok()?
        }
        let owner = self.owner();
        let owner_id = &owner.id();
        let mut rights = self
            .ownership()
            .iter()
            .map(|own| (own.rightholder().id(), own.rights()))
            .collect::<HashMap<MemberId, Rights>>();
        rights.insert(*owner_id, Rights::new_owned());
        schema.update_rights(&object, rights);
        schema.update_unstructured_ownership(&object, self.unstructured_ownership());
        schema.update_object_data(obj_id, self.data(), tx_hash);
        Ok(())
    }
}

impl Transaction for UpdateObject {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.owner().is_valid() && self.object().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let object = self.object();
        let obj_id = &object.id();
        if !schema.objects().contains(obj_id) {
            Error::no_object(&object).ok()?
        }
        let owner_id = &self.owner().id();
        let old_owner_id = &schema
            .find_owner(obj_id)
            .ok_or_else(|| Error::no_owner(&object))?;
        if !schema.unlock_object(old_owner_id, obj_id) {
            Error::locked_object(&object).ok()?
        }
        let mut rights = self
            .ownership()
            .iter()
            .map(|own| (own.rightholder().id(), own.rights()))
            .collect::<HashMap<MemberId, Rights>>();
        rights.insert(*owner_id, Rights::new_owned());
        schema.update_rights(&object, rights);
        schema.update_unstructured_ownership(&object, self.unstructured_ownership());
        schema.update_object_data(obj_id, self.data(), tx_hash);
        Ok(())
    }
}

impl Transaction for AddParticipant {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.member().is_valid() && verify_node_name(self.node_name()).is_ok()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let member = self.member();
        let member_id = &member.id();
        let node_name = self.node_name();

        for participant in schema.participants(member_id).iter() {
            if participant == node_name {
                Error::participant_already_exists(&member).ok()?
            }
        }
        schema.add_participant(member_id, node_name.to_string());
        Ok(())
    }
}

impl Transaction for OpenLot {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.lot().verify().is_ok()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let lot = self.lot();
        let mut schema = Schema::new(fork);
        let lot_id = &tx_hash;
        /*let time = schema.time().get().ok_or_else(|| Error::no_time_provider())?;
        if lot.is_started(time) {
            Error::bad_state("lot bidding period is already started").ok()?
        }*/
        if schema.lots().contains(lot_id) {
            Error::duplicate_lot(lot_id).ok()?
        }
        let requestor = self.requestor();
        let uid = &requestor.id();
        let conditions = self.conditions();

        schema.apply_checks(tx_hash, conditions.check());
        schema.set_check(tx_hash, conditions.check_seller(&requestor));
        schema.apply_checks(tx_hash, conditions.check_rights(&schema, &requestor)?);
        schema.check_result(tx_hash)?;

        for ownership in conditions.objects() {
            schema.set_published(&ownership.object().id(), lot_id);
        }
        schema.add_member_lot(uid, lot_id);
        schema.set_lot_state(lot_id, LotState::open(lot.name(), lot.price()));
        schema.add_lot(lot_id, lot, conditions);
        Ok(())
    }
}

impl Transaction for CloseLot {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self._type() == TxType::CloseLot as u8
    }

    fn execute(&self, fork: &mut Fork, _: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        /*
        let lot = schema.lots().get(lot_id).ok_or(Error::no_lot(lot_id))?;
        let time = schema.time().get().ok_or_else(|| Error::no_time_provider())?;
        if lot.is_started(time) {
            Error::bad_state("bidding period is already started").ok()?
        }*/
        let conditions = schema
            .lot_conditions()
            .get(lot_id)
            .ok_or(Error::no_lot(lot_id))?;
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;
        if state.is_completed() {
            Error::bad_state("lot is still in-progress").ok()?
        }
        schema.remove_lot_state(lot_id);
        let lot_tx = match get_transaction(&schema, lot_id)? {
            OwnershipTransactions::OpenLot(tx) => Ok(tx),
            _ => Error::unexpected_tx_type(lot_id).ok(),
        }?;
        let member_id = &lot_tx.requestor().id();
        if !schema.member_lots(member_id).contains(lot_id) {
            Error::no_permissions().ok()?
        }
        for object in conditions.objects() {
            let obj_id = &object.object().id();
            schema.set_unpublished(obj_id, lot_id);
        }
        schema.remove_member_lot(member_id, lot_id);
        schema.remove_lot(lot_id);
        Ok(())
    }
}

impl Transaction for EditLotStatus {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;
        let new_status = LotStatus::try_from(self.status())
            .map_err(|_| Error::bad_lot_status(&self.status().to_string()))?;
        if new_status != LotStatus::Rejected && new_status != LotStatus::Verified {
            Error::bad_state("lot status should be 'rejected' or 'verified'").ok()?
        }
        if !state.is_new() {
            Error::bad_state("lot status should be new").ok()?
        }
        let new_state = state.set_status(new_status);
        schema.set_lot_state(lot_id, new_state);
        Ok(())
    }
}

impl Transaction for AcquireLot {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let acquirer = self.requestor();
        let lot_tx = match get_transaction(&schema, lot_id) {
            Ok(OwnershipTransactions::OpenLot(tx)) => tx,
            _ => Error::no_lot(lot_id).ok()?,
        };
        let rightholder = lot_tx.requestor();

        // Check if lot was deleted
        if !schema.member_lots(&rightholder.id()).contains(lot_id) {
            Error::no_permissions().ok()?
        }
        let lot = lot_tx.lot();
        let price = if lot.is_private_sale() {
            lot.price()
        } else if lot.is_auction() {
            let state = schema
                .lot_states()
                .get(lot_id)
                .ok_or_else(|| Error::bad_state("Lot's state wasn't found"))?;
            if !state.is_executed() {
                Error::bad_state("Lot hasn't been executed yet").ok()?;
            }
            let max_bid = state.price();
            let mut requestor_is_the_highest_bidder = false;
            for bid_tx_hash in schema.bid_history(lot_id).iter() {
                let bid_tx = match get_private_tx(&schema, &bid_tx_hash)? {
                    OwnershipTransactions::AddBid(tx) => tx,
                    _ => unreachable!(),
                };
                if bid_tx.requestor() == acquirer && bid_tx.bid().value() == max_bid {
                    requestor_is_the_highest_bidder = true;
                    break;
                }
            }
            if !requestor_is_the_highest_bidder {
                Error::no_permissions().ok()?
            }
            max_bid
        } else {
            unreachable!()
        };

        let conditions = lot_tx.conditions();

        schema.apply_checks(tx_hash, conditions.check());
        schema.set_check(tx_hash, conditions.check_buyer(&acquirer));
        schema.set_check(tx_hash, conditions.check_seller(&rightholder));
        schema.apply_checks(tx_hash, conditions.check_rights(&schema, &rightholder)?);
        schema.check_result(tx_hash)?;

        let contract = Contract::buy(acquirer, rightholder, price, conditions);
        schema.add_contract(tx_hash, contract);
        Ok(())
    }
}

impl Transaction for PurchaseOffer {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let requestor = self.requestor();
        let rightholder = self.rightholder();
        let conditions = self.conditions();

        schema.apply_checks(tx_hash, conditions.check());
        schema.set_check(tx_hash, conditions.check_buyer(&requestor));
        schema.set_check(tx_hash, conditions.check_seller(&rightholder));
        schema.apply_checks(tx_hash, conditions.check_rights(&schema, &rightholder)?);
        schema.check_result(tx_hash)?;

        let price = self.price();
        let contract = Contract::buy(requestor, rightholder, price, conditions);
        schema.add_contract(tx_hash, contract);
        Ok(())
    }
}

impl Transaction for AddTaxInfo {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        contract.check_can_add_tax()?;

        let requestor = self.requestor();
        let payment_number = self.number();
        let payment_date = self.payment_date();
        let amount = self.amount();

        if !contract.is_member(&requestor) {
            Error::no_permissions().ok()?
        }
        if schema
            .contract_payment()
            .contains(&payment_number.to_owned())
        {
            Error::duplicate_payment().ok()?
        }
        schema.add_contract_tax(
            contract_tx_hash,
            Tax::new(requestor, payment_number, payment_date, amount),
        );

        schema.set_check(contract_tx_hash, CheckKey::TaxPaymentInfoAdded.ok());
        Ok(())
    }
}

impl Transaction for DraftContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();

        // Check if contract exists and could be drafted before other checks
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?
            .apply(Action::MakeDraft)?;

        let deed_hash = schema
            .contract_deed(contract_tx_hash)
            .ok_or(Error::deed_file_not_found(contract_tx_hash))?;
        let application_hash = schema
            .contract_application(contract_tx_hash)
            .ok_or(Error::application_file_not_found(contract_tx_hash))?;

        if &deed_hash != self.deed_tx_hash() {
            Err(Error::mismatched_deed_files())?;
        };

        if &application_hash != self.application_tx_hash() {
            Err(Error::mismatched_application_files())?;
        };

        let stored_docs: HashSet<DocumentId> =
            schema.contract_files(contract_tx_hash).keys().collect();
        // input documents list must match stored one
        if !self
            .doc_tx_hashes()
            .iter()
            .cloned()
            .collect::<HashSet<DocumentId>>()
            .eq(&stored_docs)
        {
            Error::mismatched_doc_list().ok()?
        }

        schema.set_check(contract_tx_hash, CheckKey::DocumentsMatchCondition.ok());
        schema.update_contract(contract_tx_hash, contract);
        Ok(())
    }
}

impl Transaction for RefuseContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);

        let contract = schema
            .contracts()
            .get(self.contract_tx_hash())
            .ok_or_else(|| Error::no_contract(self.contract_tx_hash()))?;

        let contract = contract.apply(Action::Refuse)?;
        schema.update_contract(self.contract_tx_hash(), contract);

        Ok(())
    }
}

impl Transaction for ConfirmContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let requestor = self.requestor();
        let contract_id = self.contract_tx_hash();
        let contract = schema
            .contracts()
            .get(contract_id)
            .ok_or_else(|| Error::no_contract(contract_id))?;

        if &schema
            .contract_deed(contract_id)
            .ok_or(Error::deed_file_not_found(contract_id))?
            != self.deed_tx_hash()
        {
            Error::mismatched_deed_files().ok()?;
        }
        if &schema
            .contract_application(contract_id)
            .ok_or(Error::application_file_not_found(contract_id))?
            != self.application_tx_hash()
        {
            Error::mismatched_application_files().ok()?;
        }

        // TODO: This should probably be moved into separate function
        // passed documents list must match stored one
        let stored_docs: HashSet<DocumentId> = schema.contract_files(contract_id).keys().collect();
        if !self
            .doc_tx_hashes()
            .iter()
            .cloned()
            .collect::<HashSet<DocumentId>>()
            .eq(&stored_docs)
        {
            Error::mismatched_doc_list().ok()?
        }

        // Rights may have changed and must be checked again
        let conditions = contract.conditions();
        let seller = contract.seller();
        schema.apply_checks(contract_id, conditions.check_rights(&schema, &seller)?);
        schema.check_result(contract_id)?;

        let contract = contract.apply(Action::Confirm(requestor))?;
        schema.update_contract(contract_id, contract);
        // TODO lock objects if they're not locked (example PurchaseOffer)
        Ok(())
    }
}

impl Transaction for AttachContractFile {
    fn verify(&self, _certificates: &HashMap<PublicKey, Certificate>) -> bool {
        self.requestor().is_valid() && self.file().verify().is_ok()
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let file = self.file();
        let contract_tx_hash = self.contract_tx_hash();
        let data_hash = crypto::hash(file.data());
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        contract.check_modifiable()?;

        schema.set_check(
            contract_tx_hash,
            CheckKey::DocumentsMatchCondition.unknown(),
        );
        schema.update_contract(contract_tx_hash, contract);

        schema.attach_file(&self.requestor().id(), tx_hash, data_hash);
        match file.file_type().try_into()? {
            AttachmentType::Deed => schema.attach_contract_deed(contract_tx_hash, *tx_hash),
            AttachmentType::Application => {
                schema.attach_contract_application(contract_tx_hash, *tx_hash)
            }
            AttachmentType::Other => schema.attach_contract_file(contract_tx_hash, tx_hash),
        }

        Ok(())
    }
}

impl Transaction for DeleteContractFiles {
    fn verify(&self, _certificates: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?
            .check_modifiable()?;

        schema.set_check(
            contract_tx_hash,
            CheckKey::DocumentsMatchCondition.unknown(),
        );

        for doc_id in self.doc_tx_hashes() {
            schema.remove_contract_file(contract_tx_hash, doc_id);
        }

        Ok(())
    }
}

impl Transaction for ApproveContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.signed_file().verify().is_ok()
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

        let contract = contract.apply(Action::Approve)?;
        schema.update_contract(contract_tx_hash, contract);

        Ok(())
    }
}

impl Transaction for RejectContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let old_contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        let status = ContractStatus::try_from(old_contract.state())?;
        if status == ContractStatus::Registering || status == ContractStatus::AwaitingUserAction {
            self.signed_file()
                .ok_or(Error::empty_transaction_param("signed_file"))?
                .verify()?;
        };
        let new_contract = old_contract.apply(Action::Reject)?;
        schema.update_contract(contract_tx_hash, new_contract);
        Ok(())
    }
}

impl Transaction for UpdateContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        // TODO
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let old_contract = schema
            .contracts()
            .get(self.contract_tx_hash())
            .ok_or_else(|| Error::no_contract(self.contract_tx_hash()))?;
        let requestor = self.requestor();
        if !old_contract.is_member(&requestor) {
            Error::no_permissions().ok()?
        }
        let conditions = self.conditions();

        schema.apply_checks(contract_tx_hash, conditions.check());
        schema.set_check(
            contract_tx_hash,
            conditions.check_buyer(&old_contract.buyer()),
        );
        schema.set_check(
            contract_tx_hash,
            conditions.check_seller(&old_contract.seller()),
        );
        schema.apply_checks(
            contract_tx_hash,
            conditions.check_rights(&schema, &old_contract.seller())?,
        );
        schema.check_result(contract_tx_hash)?;

        let price = self.price();
        let contract = old_contract.apply(Action::Update { price, conditions })?;
        schema.update_contract(contract_tx_hash, contract);
        schema.clear_contract_files(contract_tx_hash);
        Ok(())
    }
}

impl Transaction for AddBid {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        let type_valid = self._type() == TxType::AddBid as u8;

        type_valid && self.requestor().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let lot = schema
            .lots()
            .get(lot_id)
            .ok_or_else(|| Error::no_lot(lot_id))?;
        if !lot.is_auction() {
            Error::action_refused("lot can't accept bids").ok()?
        }
        /*let time = schema.time().get().ok_or(NoTimeProviderError.into_err())?;
        if !lot.is_open_for_bids(time) {
            Error::out_of_time(time).ok()?
        }*/
        let uid = &self.requestor().id();
        if schema.member_lots(uid).contains(lot_id) {
            Error::no_permissions().ok()?
        }
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or(Error::bad_state("lot state wasn't found"))?;
        if !state.is_verified() {
            Error::bad_state("lot hasn't been verified").ok()?
        }
        if state.price() >= self.bid().value() {
            Error::bad_state("low bid value").ok()?
        }
        schema.put_bid_tx(lot_id, tx_hash.clone());
        Ok(())
    }
}

impl Transaction for PublishBids {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let lot = schema
            .lots()
            .get(lot_id)
            .ok_or_else(|| Error::no_lot(lot_id))?;
        if !lot.is_auction() {
            Error::action_refused("lot can't accept bids").ok()?
        }
        /*let time = schema.time().get().ok_or_else(|| Error::no_time_provider())?;
        if !lot.is_started(time) {
            Error::bad_state("lot bidding period isn't started yet").ok()?
        }*/
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or(Error::bad_state("lot state wasn't found"))?;
        if !state.is_verified() {
            Error::bad_state("lot hasn't been verified").ok()?
        }

        /* TODO should be checked only in block precommit
        // checking all private bids on validator
        if schema.is_validator(executor) {
            let num_published_bids = schema.bids(lot_id).len() as usize;
            let private_bids = schema.bid_history(lot_id)
                .iter()
                .skip(num_published_bids)
                .collect::<Vec<Hash>>();
            let bids = self.bids();
            let num_bids = bids.len();
            for (i, tx_hash) in private_bids.into_iter().enumerate() {
                if i >= num_bids {
                    break;
                }
                let tx = match get_private_tx(&schema, &tx_hash)? {
                    OwnershipTransactions::AddBid(tx) => Ok(tx),
                    _ => Error::unexpected_tx_type(tx_hash).ok(),
                }?;
                let private_bid = tx.bid().value();
                if bids[i] != private_bid {
                    return Err(MissedBidError(tx_hash).into_err());
                    Error::missed_bid(tx_hash).ok()?
                }
            }
        }*/

        let current_price = state.price();
        let mut max_bid_value = current_price;
        for bid_value in self.bids() {
            if current_price >= bid_value {
                Error::bad_state("low bid value").ok()?
            }
            schema.add_bid(lot_id, Bid::new(bid_value));
            max_bid_value = u64::max(max_bid_value, bid_value);
        }

        /*let state = if lot.is_finished(now) {
            state.set_status(LotStatus::Completed)
        } else {
            state
        }*/

        let newstate = state.set_price(max_bid_value);
        schema.set_lot_state(lot_id, newstate);
        Ok(())
    }
}

impl Transaction for ExtendLotPeriod {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let requestor = self.requestor();

        if !schema.member_lots(&requestor.id()).contains(lot_id) {
            Error::no_permissions().ok()?
        }

        let lot = schema
            .lots()
            .get(lot_id)
            .ok_or_else(|| Error::no_lot(lot_id))?;
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or_else(|| Error::bad_state("Failed to find lot state"))?;

        if self.new_expiration_date() <= lot.closing_time() {
            Error::bad_lot_time_extension().ok()?
        }
        if !(state.is_new() || state.is_verified()) {
            Error::bad_lot_status(&state.status().to_string()).ok()?
        }
        let conditions = schema
            .lot_conditions()
            .get(lot_id)
            .ok_or_else(|| Error::bad_state("Failed to find lot conditions"))?;

        let lot = Lot::new(
            lot.name(),
            lot.desc(),
            lot.price(),
            lot.sale_type(),
            lot.opening_time(),
            self.new_expiration_date(),
        );

        schema.remove_lot(lot_id);
        schema.add_lot(lot_id, lot, conditions);
        Ok(())
    }
}

impl Transaction for ExecuteLot {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let lot = schema
            .lots()
            .get(lot_id)
            .ok_or_else(|| Error::no_lot(lot_id))?;
        if !lot.is_auction() {
            Error::action_refused("lot can't be executed").ok()?
        }
        /*let time = schema.time().get().ok_or(NoTimeProviderError.into_err())?;
        if !lot.is_finished(time) {
            Error::bad_state("lot bidding period hasn't been finished yet").ok()?
        }*/
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or(Error::bad_state("lot state wasn't found"))?;
        /*if !state.is_completed() {
            Error::bad_state("lot bids haven't been published yet").ok()?
        }*/
        let newstate = state.set_status(LotStatus::Executed);
        schema.set_lot_state(lot_id, newstate);
        Ok(())
    }
}

impl Transaction for RegisterContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let old_contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        let new_contract = old_contract.apply(Action::Register)?;
        schema.update_contract(contract_tx_hash, new_contract);
        Ok(())
    }
}

impl Transaction for AwaitUserActionContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let old_contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        let new_contract = old_contract.apply(Action::AwaitUserAction)?;
        schema.update_contract(contract_tx_hash, new_contract);
        Ok(())
    }
}

impl Transaction for SignContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let old_contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

        let new_contract = old_contract.apply(Action::Sign(self.requestor()))?;

        // let deed_hash = &schema
        //     .contract_deed(contract_tx_hash)
        //     .ok_or(Error::deed_file_not_found(contract_tx_hash))?;
        // let deed_attachment: Attachment = {
        //     let doc_tx = get_private_tx(&schema, deed_hash)?;
        //     match doc_tx {
        //         OwnershipTransactions::AttachContractFile(tx) => Ok(tx.file()),
        //         _ => Error::unexpected_tx_type(deed_hash).ok(),
        //     }
        // }?;
        // let deed_data = deed_attachment.data();
        // let deed_sign = self.deed_sign();
        // member_matches_sign(&self.requestor(), &deed_sign)?;
        // deed_sign.verify_data(deed_data)?;
        //
        // let application_hash = &schema
        //     .contract_deed(contract_tx_hash)
        //     .ok_or(Error::deed_file_not_found(contract_tx_hash))?;
        // let application_attachment: Attachment = {
        //     let doc_tx = get_private_tx(&schema, application_hash)?;
        //     match doc_tx {
        //         OwnershipTransactions::AttachContractFile(tx) => Ok(tx.file()),
        //         _ => Error::unexpected_tx_type(application_hash).ok(),
        //     }
        // }?;
        // let application_data = application_attachment.data();
        // let application_sign = self.application_sign();
        // member_matches_sign(&self.requestor(), &application_sign)?;
        // application_sign.verify_data(application_data)?;

        schema.update_contract(contract_tx_hash, new_contract);
        Ok(())
    }
}

impl Transaction for SubmitChecks {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        if !schema.contracts().contains(contract_tx_hash) {
            Err(Error::no_contract(contract_tx_hash))?;
        };
        for check in self.checks() {
            schema.set_check(contract_tx_hash, check);
        }
        Ok(())
    }
}

impl Transaction for ContractReferenceNumber {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        if !schema.contracts().contains(contract_tx_hash) {
            Err(Error::no_contract(contract_tx_hash))?;
        };
        schema.set_contract_reference_number(contract_tx_hash, self.reference_number().to_string());
        Ok(())
    }
}
