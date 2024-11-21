use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use chrono::{DateTime, Utc};
use rand::{thread_rng, RngCore};

use blockp_core::blockchain::{
    Blockchain, ExecutionError, ExecutionResult, PreExecutionError, PreExecutionResult,
    Transaction, TransactionSet,
};
use blockp_core::crypto::{get_cert_from_detached_sign, Certificate, Hash, PublicKey};
use blockp_core::messages::RawMessage;
use blockp_core::storage::{Fork, Snapshot};

use crate::data::attachment::{Attachment, AttachmentType, DocumentId, Sign};
use crate::data::conditions::{Check, CheckKey, Conditions};
use crate::data::contract::{Action, BuyerSeller, Contract, ContractId, ContractStatus};
use crate::data::cost::Cost;
use crate::data::lot::{Bid, Lot, LotId, LotState, LotStatus, SaleType};
use crate::data::member::MemberIdentity;
use crate::data::object::ObjectIdentity;
use crate::data::ownership::{Ownership, OwnershipUnstructured, Rights};
#[cfg(feature = "internal_api")]
use crate::data::payment::PaymentStatus;
use crate::data::payment::{Calculation, PaymentDetail, PaymentDetailsWrapper};
use crate::data::strings::verify_node_name;
use crate::error::{self, Error};
use crate::schema::Schema;
use crate::EsiaAuth;

impl From<Error> for ExecutionError {
    fn from(err: Error) -> Self {
        ExecutionError::with_description(err.code(), err.info())
    }
}

impl From<Error> for PreExecutionError {
    fn from(err: Error) -> Self {
        PreExecutionError::with_description(err.code(), err.info())
    }
}

#[cfg(feature = "internal_api")]
pub fn add_object(
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<Ownership>,
    ownership_unstructured: Vec<OwnershipUnstructured>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AddObject::new(
        salt(),
        TxType::AddObject as u8,
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
    object: ObjectIdentity,
    data: &str,
    ownership: Vec<Ownership>,
    ownership_unstructured: Vec<OwnershipUnstructured>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    UpdateObject::new(
        salt(),
        TxType::UpdateObject as u8,
        object,
        data,
        ownership,
        ownership_unstructured,
        cert,
    )
    .into()
}

// pub fn attach_file(
//     requestor: MemberIdentity,
//     file: Attachment,
//     cert: &Certificate,
//     members: Vec<MemberIdentity>,
//     share: Vec<PublicKey>,
// ) -> Box<dyn Transaction> {
//     AttachFile::new(
//         salt(),
//         TxType::AttachFile as u8,
//         requestor,
//         file,
//         members,
//         share,
//         cert,
//     )
//     .into()
// }

// pub fn delete_files(
//     requestor: MemberIdentity,
//     doc_hashes: &[DocumentId],
//     cert: &Certificate,
// ) -> Box<dyn Transaction> {
//     DeleteFiles::new(
//         salt(),
//         TxType::DeleteFiles as u8,
//         requestor,
//         doc_hashes,
//         cert,
//     )
//     .into()
// }

// pub fn add_attachment_sign(
//     requestor: MemberIdentity,
//     doc_tx_hash: &Hash,
//     sign: Sign,
//     cert: &Certificate,
//     share: Vec<PublicKey>,
// ) -> Box<dyn Transaction> {
//     AddAttachmentSign::new(
//         salt(),
//         TxType::AddAttachmentSign as u8,
//         requestor,
//         doc_tx_hash,
//         sign,
//         share,
//         cert,
//     )
//     .into()
// }

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
    CloseLot::new(salt(), TxType::CloseLot as u8, requestor, lot_id, cert).into()
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
pub fn lot_undefined(lot_id: &LotId, admit: bool, cert: &Certificate) -> Box<dyn Transaction> {
    LotUndefined::new(salt(), TxType::LotUndefined as u8, lot_id, admit, cert).into()
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
    buyer: MemberIdentity,
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
        buyer,
        rightholder,
        price.into(),
        conditions,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn draft_contract(
    contract_tx_hash: &ContractId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    DraftContract::new(
        0,
        TxType::DraftContract as u8,
        contract_tx_hash,
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
    contract_correspondence: Option<String>,
    objects_correspondence: Option<String>,
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
        contract_correspondence,
        objects_correspondence,
        share,
        cert,
    )
    .into()
}

pub fn attach_contract_other_file(
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    file: Attachment,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AttachContractOtherFile::new(
        0,
        TxType::AttachContractOtherFile as u8,
        requestor,
        contract_tx_hash,
        file,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn attach_contract_main_file(
    contract_tx_hash: &ContractId,
    file: Attachment,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    AttachContractMainFile::new(
        0,
        TxType::AttachContractMainFile as u8,
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
    attachment: Option<Attachment>,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ApproveContract::new(
        0,
        TxType::ApproveContract as u8,
        contract_tx_hash,
        attachment,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn reject_contract(
    contract_tx_hash: &ContractId,
    reason: &str,
    attachment: Option<Attachment>,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    RejectContract::new(
        0,
        TxType::RejectContract as u8,
        contract_tx_hash,
        reason,
        attachment,
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
        salt(),
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
        salt(),
        TxType::AwaitUserActionContract as u8,
        contract_tx_hash,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn contract_submit_checks(
    contract_tx_hash: &ContractId,
    checks: Vec<Check>,
    is_undef: bool,
    reference_number: Option<String>,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ContractSubmitChecks::new(
        0,
        TxType::ContractSubmitChecks as u8,
        contract_tx_hash,
        checks,
        is_undef,
        reference_number,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn lot_submit_checks(
    contract_tx_hash: &ContractId,
    checks: Vec<Check>,
    is_undef: bool,
    reference_number: Option<String>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    LotSubmitChecks::new(
        0,
        TxType::LotSubmitChecks as u8,
        contract_tx_hash,
        checks,
        is_undef,
        reference_number,
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

pub fn tax_request(
    requestor: MemberIdentity,
    contract_tx_hash: &ContractId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    TaxRequest::new(
        salt(),
        TxType::TaxRequest as u8,
        requestor,
        contract_tx_hash,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn tax_contract_calculation(
    contract_tx_hash: &ContractId,
    calculations: Vec<Calculation>,
    reference_number: Option<String>,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    TaxContractCalculation::new(
        salt(),
        TxType::TaxContractCalculation as u8,
        contract_tx_hash,
        calculations,
        reference_number,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn tax_lot_calculation(
    lot_tx_hash: &LotId,
    calculations: Vec<Calculation>,
    reference_number: Option<String>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    TaxLotCalculation::new(
        salt(),
        TxType::TaxLotCalculation as u8,
        lot_tx_hash,
        calculations,
        reference_number,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn tax_with_payment_details(
    contract_tx_hash: &ContractId,
    payment_details: Vec<PaymentDetail>,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    TaxWithPaymentDetails::new(
        salt(),
        TxType::TaxWithPaymentDetails as u8,
        contract_tx_hash,
        payment_details,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn tax_status(
    contract_tx_hash: &ContractId,
    payment_id: &str,
    status: PaymentStatus,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    TaxStatus::new(
        0,
        TxType::TaxStatus as u8,
        contract_tx_hash,
        payment_id,
        status as u8,
        share,
        cert,
    )
    .into()
}

pub fn contract_confirm_create(
    requestor_id: MemberIdentity,
    contract_id: &ContractId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ContractConfirmCreate::new(
        salt(),
        TxType::ContractConfirmCreate as u8,
        requestor_id,
        contract_id,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn contract_unconfirm_create(
    member_id: MemberIdentity,
    contract_id: &ContractId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ContractUnconfirmCreate::new(
        salt(),
        TxType::ContractUnconfirmCreate as u8,
        member_id,
        contract_id,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn contract_undefined(
    contract_id: &ContractId,
    admit: bool,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ContractUndefined::new(
        salt(),
        TxType::ContractUndefined as u8,
        contract_id,
        admit,
        share,
        cert,
    )
    .into()
}

#[cfg(feature = "internal_api")]
pub fn contract_new(
    contract_id: &ContractId,
    share: Vec<PublicKey>,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    ContractNew::new(salt(), TxType::ContractNew as u8, contract_id, share, cert).into()
}

pub fn member_token(
    member: MemberIdentity,
    token: &str,
    oid: &str,
    cert: &Certificate,
) -> Box<dyn Transaction> {
    MemberToken::new(0, TxType::MemberToken as u8, member, token, oid, cert).into()
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

#[allow(unreachable_code)]
fn member_matches_sign(_member: &MemberIdentity, _sign: &Sign) -> Result<(), Error> {
    const OGRN_OID: &'static str = "1.2.643.100.1";
    const OGRNIP_OID: &'static str = "1.2.643.100.5";
    const SNILS_OID: &'static str = "1.2.643.100.3";

    // Look up https://aj.srvdev.ru/browse/FIPSOP-1007
    return Ok(());

    let certificate = get_cert_from_detached_sign(_sign.data())
        .map_err(|_| Error::bad_signature("unable to decode"))?
        .ok_or_else(|| Error::bad_signature("unable to extract certificate"))?;

    let certificate_member = match _member.class() {
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

    if certificate_member == _member.number() {
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
    // AddObjectRequest = 1,
    // AddObjectGroupRequest = 2,
    AddObject = 3,
    UpdateObject = 4,
    // AttachFile = 5,
    // DeleteFiles = 6,
    // AddAttachmentSign = 7,
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
    AttachContractOtherFile = 23,
    DeleteContractFiles = 24,
    ApproveContract = 25,
    RejectContract = 26,
    UpdateContract = 27,
    RegisterContract = 28,
    AwaitUserActionContract = 29,
    SignContract = 30,
    ContractSubmitChecks = 31,
    // AddTaxInfo = 32,
    ContractReferenceNumber = 33,
    ExtendLotPeriod = 34,
    LotUndefined = 35,
    TaxRequest = 36,
    TaxLotCalculation = 37,
    TaxContractCalculation = 38,
    TaxWithPaymentDetails = 39,
    TaxStatus = 40,
    LotSubmitChecks = 41,
    ContractUndefined = 42,
    MemberToken = 43,
    ContractConfirmCreate = 44,
    ContractUnconfirmCreate = 45,
    ContractNew = 46,
    AttachContractMainFile = 47,
}

transactions! {
    pub OwnershipTransactions {
        const SERVICE_ID = crate::service::SERVICE_ID;

        struct AddObject {
            _type: u8,
            object: ObjectIdentity,
            data: &str,
            ownership: Vec<Ownership>,
            unstructured_ownership: Vec<OwnershipUnstructured>,
        }

        struct UpdateObject {
            _type: u8,
            object: ObjectIdentity,
            data: &str,
            ownership: Vec<Ownership>,
            unstructured_ownership: Vec<OwnershipUnstructured>,
        }

        // struct AttachFile {
        //     _type: u8,
        //     requestor: MemberIdentity,
        //     file: Attachment,
        //     members: Vec<MemberIdentity>,
        //     share: Vec<PublicKey>,
        // }

        // struct DeleteFiles {
        //     _type: u8,
        //     requestor: MemberIdentity,
        //     doc_tx_hashes: &[DocumentId],
        // }

        // struct AddAttachmentSign {
        //     _type: u8,
        //     requestor: MemberIdentity,
        //     doc_tx_hash: &DocumentId,
        //     sign: Sign,
        //     share: Vec<PublicKey>,
        // }

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
            buyer: MemberIdentity,
            rightholder: MemberIdentity,
            price: u64,
            conditions: Conditions,
            share: Vec<PublicKey>,
        }

        struct DraftContract {
            _type: u8,
            contract_tx_hash: &ContractId,
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
            share: Vec<PublicKey>,
        }

        struct AttachContractOtherFile {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            file: Attachment,
            share: Vec<PublicKey>,
        }

        struct AttachContractMainFile {
            _type: u8,
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
            attachment: Option<Attachment>,
            share: Vec<PublicKey>,
        }

        struct RejectContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            reason: &str,
            attachment: Option<Attachment>,
            share: Vec<PublicKey>,
        }

        struct UpdateContract {
            _type: u8,
            contract_tx_hash: &ContractId,
            requestor: MemberIdentity,
            price: u64,
            conditions: Conditions,
            contract_correspondence: Option<String>,
            objects_correspondence: Option<String>,
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

        struct ContractSubmitChecks {
            _type: u8,
            contract_tx_hash: &ContractId,
            checks: Vec<Check>,
            is_undef: bool,
            reference_number: Option<String>,
            share: Vec<PublicKey>,
        }

        struct ContractReferenceNumber {
            _type: u8,
            contract_tx_hash: &ContractId,
            reference_number: &str,
            share: Vec<PublicKey>,
        }

        struct LotUndefined {
            _type: u8,
            lot_tx_hash: &LotId,
            admit: bool
        }

        struct TaxRequest {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            share: Vec<PublicKey>,
        }

        struct TaxLotCalculation {
            _type: u8,
            lot_tx_hash: &LotId,
            calculations: Vec<Calculation>,
            reference_number: Option<String>
        }

        struct TaxContractCalculation {
            _type: u8,
            contract_tx_hash: &ContractId,
            calculations: Vec<Calculation>,
            reference_number: Option<String>,
            share: Vec<PublicKey>,
        }

        struct TaxWithPaymentDetails {
            _type: u8,
            contract_tx_hash: &ContractId,
            payment_details: Vec<PaymentDetail>,
            share: Vec<PublicKey>,
        }

        struct TaxStatus {
            _type: u8,
            contract_tx_hash: &ContractId,
            payment_id: &str,
            status: u8,
            share: Vec<PublicKey>,
        }

        struct LotSubmitChecks {
            _type: u8,
            lot_tx_hash: &LotId,
            checks: Vec<Check>,
            is_undef: bool,
            reference_number: Option<String>
        }

        struct ContractUndefined {
            _type: u8,
            contract_tx_hash: &ContractId,
            admit: bool,
            share: Vec<PublicKey>,
        }

        struct MemberToken {
            _type: u8,
            member: MemberIdentity,
            token: &str,
            oid: &str,
        }

        struct ContractConfirmCreate {
            _type: u8,
            requestor: MemberIdentity,
            contract_tx_hash: &ContractId,
            share: Vec<PublicKey>,
        }

        struct ContractUnconfirmCreate {
            _type: u8,
            member: MemberIdentity,
            contract_tx_hash: &ContractId,
            share: Vec<PublicKey>,
        }

        struct ContractNew {
            _type: u8,
            contract_tx_hash: &ContractId,
            share: Vec<PublicKey>,
        }
    }
}

// impl Transaction for AddAttachmentSign {
//     fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
//         self.requestor().is_valid() && self.sign().verify().is_ok()
//     }
//
//     fn participants(&self) -> Vec<PublicKey> {
//         self.share()
//     }
//
//     fn execute(&self, fork: &mut Fork, _hash: &Hash, _: &PublicKey) -> ExecutionResult {
//         let mut schema = Schema::new(fork);
//         let doc_tx_hash = self.doc_tx_hash();
//         let uid = &self.requestor().id();
//
//         if !schema.attachments(uid).contains(doc_tx_hash) {
//             Error::no_attachment(doc_tx_hash).ok()?
//         }
//
//         let attachment: Attachment = {
//             let doc_tx = get_private_tx(&schema, doc_tx_hash)?;
//             match doc_tx {
//                 // OwnershipTransactions::AttachFile(tx) if tx.requestor().id() == *uid => {
//                 //     Ok(tx.file())
//                 // }
//                 OwnershipTransactions::AttachContractFile(tx) => Ok(tx.file()),
//                 _ => Error::unexpected_tx_type(doc_tx_hash).ok(),
//             }
//         }?;
//         let doc_data = attachment.data();
//         let sign = self.sign();
//         member_matches_sign(&self.requestor(), &sign)?;
//         sign.verify_data(doc_data)?;
//
//         Ok(schema.add_attachment_sign(doc_tx_hash, uid, sign))
//     }
// }

// impl Transaction for AttachFile {
//     fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
//         self.requestor().is_valid()
//             && self.file().verify().is_ok()
//             && !contains_diplicates(self.members())
//             && !contains_diplicates(self.share())
//     }
//
//     fn participants(&self) -> Vec<PublicKey> {
//         self.share()
//     }
//
//     fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
//         let mut schema = Schema::new(fork);
//         let data_hash = crypto::hash(self.file().data());
//         schema.attach_file(&self.requestor().id(), tx_hash, data_hash);
//
//         for p in self.members().iter() {
//             schema.attach_file(&p.id(), tx_hash, data_hash);
//         }
//         Ok(())
//     }
// }

// impl Transaction for DeleteFiles {
//     fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
//         self.requestor().is_valid()
//     }
//
//     fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
//         let mut schema = Schema::new(fork);
//         let uid = &self.requestor().id();
//
//         let docs = self.doc_tx_hashes();
//
//         for doc_tx_hash in docs {
//             if !schema.attachments(uid).contains(doc_tx_hash) {
//                 Error::no_attachment(doc_tx_hash).ok()?
//             }
//             let doc_tx = get_private_tx(&schema, doc_tx_hash)?;
//             match doc_tx {
//                 OwnershipTransactions::AttachFile(tx) if tx.requestor().id() == *uid => Ok(()),
//                 _ => Error::unexpected_tx_type(tx_hash).ok(),
//             }?;
//
//             schema.remove_attachment_sign(&uid, doc_tx_hash);
//             schema.remove_file(&uid, doc_tx_hash);
//         }
//
//         Ok(())
//     }
// }

impl Transaction for AddObject {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.object().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let object = self.object();
        let obj_id = &object.id();
        if schema.objects().contains(obj_id) {
            Error::duplicate_object(&object).ok()?
        }

        let rights = self
            .ownership()
            .into_iter()
            .map(|own| (own.rightholder(), own.rights()))
            .collect::<HashMap<MemberIdentity, Rights>>();

        schema.update_rights(&object, rights);
        schema.update_unstructured_ownership(&object, self.unstructured_ownership());
        schema.invalidate_published_lots(obj_id);
        schema.invalidate_published_contracts(obj_id);
        schema.update_object_data(obj_id, self.data(), tx_hash, object);

        Ok(())
    }
}

impl Transaction for UpdateObject {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.object().is_valid()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let object = self.object();
        let obj_id = &object.id();
        if !schema.objects().contains(obj_id) {
            Error::no_object(&object).ok()?
        }

        let rights = self
            .ownership()
            .iter()
            .map(|own| (own.rightholder(), own.rights()))
            .collect::<HashMap<MemberIdentity, Rights>>();

        schema.update_rights(&object, rights);
        schema.update_unstructured_ownership(&object, self.unstructured_ownership());
        schema.invalidate_published_lots(obj_id);
        schema.invalidate_published_contracts(obj_id);
        schema.update_object_data(obj_id, self.data(), tx_hash, object);
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
        schema.add_lot(**lot_id, lot, conditions);
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

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
        // schema.remove_lot_state(lot_id);
        let requestor_id = &self.requestor().id();
        if !schema.member_lots(requestor_id).contains(lot_id) {
            Error::no_permissions().ok()?
        }
        for object in conditions.objects() {
            let obj_id = &object.object().id();
            schema.set_unpublished(obj_id, lot_id);
        }
        // schema.remove_lot(lot_id);
        schema.remove_member_lot(requestor_id, lot_id);
        let new_state = state.set_status_closed();
        schema.set_lot_state(lot_id, new_state);
        schema.remove_lot_data(lot_id);
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for EditLotStatus {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _: &Hash, _: &PublicKey) -> ExecutionResult {
        // let mut schema = Schema::new(fork);
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;
        let new_status = LotStatus::try_from(self.status())
            .map_err(|_| Error::bad_lot_status(&self.status().to_string()))?;

        if !state.is_new() && !state.is_verified() {
            Error::bad_state("lot status should be 'new' or 'verified'").ok()?;
        }

        let new_state = match new_status {
            LotStatus::Rejected => {
                let conditions = schema
                    .lot_conditions()
                    .get(lot_id)
                    .ok_or(Error::no_lot(lot_id))?;

                for object in conditions.objects() {
                    let obj_id = &object.object().id();
                    schema.set_unpublished(obj_id, lot_id);
                }

                state.set_status_rejected()
            }
            LotStatus::Verified => state.set_status_verified(),
            LotStatus::Closed => {
                let conditions = schema
                    .lot_conditions()
                    .get(lot_id)
                    .ok_or(Error::no_lot(lot_id))?;

                for object in conditions.objects() {
                    let obj_id = &object.object().id();
                    schema.set_unpublished(obj_id, lot_id);
                }

                schema.remove_lot_data(lot_id);
                state.set_status_closed()
            }
            _ => {
                Error::bad_state("new lot status must be 'rejected', 'verified', 'closed'").ok()?
            }
        };

        schema.set_lot_state(lot_id, new_state);
        Ok(())
    }
}

impl Transaction for LotUndefined {
    fn verify(&self, _certificates: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_id = self.lot_tx_hash();
        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or_else(|| Error::bad_state("lot state wasn't found"))?;

        if state.is_closed() {
            Error::lot_is_closed(lot_id).ok()?
        } else if state.is_rejected() {
            Error::lot_is_rejected(lot_id).ok()?
        }

        if !state.undefined() {
            Error::bad_state("lot status should be 'undefined'").ok()?
        }

        let new_state = match self.admit() {
            true => state.set_undefined(false),
            false => state.set_status_rejected(),
        };

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
        // TODO not best solution for determining the right holder.
        let lot_tx = match get_transaction(&schema, lot_id) {
            Ok(OwnershipTransactions::OpenLot(tx)) => tx,
            _ => Error::no_lot(lot_id).ok()?,
        };
        let rightholder = lot_tx.requestor();

        if schema.member_lots(&acquirer.id()).contains(lot_id) {
            Error::no_permissions().ok()?
        }

        let lot = schema
            .lots()
            .get(lot_id)
            .ok_or_else(|| Error::no_lot(lot_id))?;

        let state = schema
            .lot_states()
            .get(lot_id)
            .ok_or_else(|| Error::bad_state("Lot's state wasn't found"))?;

        if state.undefined() {
            Error::lot_is_undefined(lot_id).ok()?;
        }

        let price = match SaleType::try_from(lot.sale_type())
            .map_err(|_| Error::internal_bad_struct("sale_type"))?
        {
            SaleType::PrivateSale => {
                if !state.is_verified() {
                    Error::bad_state("Lot hasn't been verified yet").ok()?;
                };
                lot.price()
            }
            SaleType::Auction => {
                if !state.is_executed() {
                    Error::bad_state("Lot hasn't been executed yet").ok()?;
                };

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

                //TODO нужно провести процедуру закрытия лота т.е. удалить лишние данные
                let new_state = state.set_status_closed();
                schema.set_lot_state(lot_id, new_state);

                max_bid
            }
        };

        let conditions = schema
            .lot_conditions()
            .get(lot_id)
            .ok_or_else(|| Error::no_lot(lot_id))?;

        schema.set_check(tx_hash, conditions.check_buyer(&acquirer));
        schema.check_result(tx_hash)?;

        for ownership in conditions.objects() {
            schema.set_published_contract(&ownership.object().id(), tx_hash);
        }
        let contract = Contract::buy(acquirer, rightholder, price, conditions);
        schema.add_contract(tx_hash, contract);

        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for PurchaseOffer {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.buyer() != self.rightholder()
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let buyer = self.buyer();
        let rightholder = self.rightholder();
        let conditions = self.conditions();

        let is_buyer = match self.requestor() {
            member_id if member_id == buyer => true,
            member_id if member_id == rightholder => false,
            _ => Error::no_permissions().ok()?,
        };

        schema.apply_checks(tx_hash, conditions.check());
        schema.set_check(tx_hash, conditions.check_buyer(&buyer));
        schema.set_check(tx_hash, conditions.check_seller(&rightholder));
        schema.apply_checks(tx_hash, conditions.check_rights(&schema, &rightholder)?);
        schema.check_result(tx_hash)?;

        for ownership in conditions.objects() {
            schema.set_published_contract(&ownership.object().id(), tx_hash);
        }

        let contract = if is_buyer {
            Contract::buy(buyer, rightholder, self.price(), conditions)
        } else {
            Contract::sell(buyer, rightholder, self.price(), conditions)
        };

        schema.add_contract(tx_hash, contract);
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

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

        schema
            .contract_deed(contract_tx_hash)
            .ok_or(Error::deed_file_not_found(contract_tx_hash))?;
        schema
            .contract_application(contract_tx_hash)
            .ok_or(Error::application_file_not_found(contract_tx_hash))?;

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

        let contract_tx_hash = self.contract_tx_hash();

        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(self.contract_tx_hash()))?;

        let contract = contract.apply(Action::Refuse)?;
        for ownership in contract.conditions().objects() {
            schema.set_unpublished_contract(&ownership.object().id(), contract_tx_hash);
        }
        schema.update_contract(self.contract_tx_hash(), contract);

        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

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

        if contract.is_undefined() {
            Error::contract_is_undefined(contract_id).ok()?;
        }

        if &schema
            .contract_deed(contract_id)
            .ok_or(Error::deed_file_not_found(contract_id))?
            .tx_hash()
            != &self.deed_tx_hash()
        {
            Error::mismatched_deed_files().ok()?;
        }
        if &schema
            .contract_application(contract_id)
            .ok_or(Error::application_file_not_found(contract_id))?
            .tx_hash()
            != &self.application_tx_hash()
        {
            Error::mismatched_application_files().ok()?;
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

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for AttachContractOtherFile {
    fn verify(&self, _certificates: &HashMap<PublicKey, Certificate>) -> bool {
        self.requestor().is_valid()
            && self.file().verify().is_ok()
            && self.file().metadata().file_type() == AttachmentType::Other as u8
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let file = self.file();

        let contract_tx_hash = self.contract_tx_hash();
        // let data_hash = crypto::hash(file.data());
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

        if !contract.is_member(&self.requestor()) {
            Err(Error::no_permissions())?
        }

        schema.set_check(
            contract_tx_hash,
            CheckKey::DocumentsMatchCondition.unknown(),
        );

        // schema.attach_file(&self.requestor().id(), tx_hash, data_hash);
        let file_metadata = file.metadata();
        match file_metadata.file_type().try_into()? {
            AttachmentType::Other => {
                let status = ContractStatus::try_from(contract.state())?;
                match status {
                    ContractStatus::New => {}
                    ContractStatus::Draft(_) => {}
                    ContractStatus::Confirmed(_) => {}
                    ContractStatus::Signed => {}
                    x => Error::bad_contract_state(x, "attaching file to contract").ok()?,
                }
                schema.attach_contract_file(contract_tx_hash, tx_hash, file_metadata)
            }
            _ => unreachable!(),
        }
        // schema.update_contract(contract_tx_hash, contract);
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for AttachContractMainFile {
    fn verify(&self, _certificates: &HashMap<PublicKey, Certificate>) -> bool {
        self.file().verify().is_ok()
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let file = self.file();
        let contract_tx_hash = self.contract_tx_hash();
        // let data_hash = crypto::hash(file.data());
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

        schema.set_check(
            contract_tx_hash,
            CheckKey::DocumentsMatchCondition.unknown(),
        );

        // schema.attach_file(&self.requestor().id(), tx_hash, data_hash);
        let file_metadata = file.metadata();
        match file_metadata.file_type().try_into()? {
            AttachmentType::Deed => {
                if !contract.check_modifiable()? {
                    let status = ContractStatus::try_from(contract.state())?;
                    Error::bad_contract_state(status, "attaching deed file to contract").ok()?;
                };
                schema.attach_contract_deed(contract_tx_hash, tx_hash, file_metadata)
            }
            AttachmentType::Application => {
                if !contract.check_modifiable()? {
                    let status = ContractStatus::try_from(contract.state())?;
                    Error::bad_contract_state(status, "attaching application file to contract")
                        .ok()?;
                };
                schema.attach_contract_application(contract_tx_hash, tx_hash, file_metadata)
            }
            AttachmentType::Notification => {
                // let status = ContractStatus::try_from(contract.state())?;
                // match status {
                //     ContractStatus::New => {}
                //     ContractStatus::Draft(_) => {}
                //     ContractStatus::Confirmed(_) => {}
                //     ContractStatus::Signed => {}
                //     x => Error::bad_contract_state(x, "attaching file to contract").ok()?,
                // }
                schema.attach_contract_notification(contract_tx_hash, tx_hash, file_metadata)
            }
            AttachmentType::Other => {
                schema.attach_contract_file(contract_tx_hash, tx_hash, file_metadata)
            }
        }
        // schema.update_contract(contract_tx_hash, contract);
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
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

        if !contract.is_member(&self.requestor()) {
            Err(Error::no_permissions())?
        }

        let status = ContractStatus::try_from(contract.state())?;
        match status {
            ContractStatus::New => {}
            ContractStatus::Draft(_) => {}
            ContractStatus::Confirmed(_) => {}
            ContractStatus::Signed => {}
            x => Error::bad_contract_state(x, "deleting file from contract").ok()?,
        }

        for doc_id in self.doc_tx_hashes() {
            if !schema.contract_files(contract_tx_hash).contains(doc_id) {
                Err(Error::no_attachment(doc_id))?
            }
            schema.remove_contract_file(contract_tx_hash, doc_id);
        }

        schema.set_check(
            contract_tx_hash,
            CheckKey::DocumentsMatchCondition.unknown(),
        );
        schema.update_contract(contract_tx_hash, contract);

        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for ApproveContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.attachment()
            .map(|attach| attach.verify())
            .transpose()
            .is_ok()
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

        let contract = contract.apply(Action::Approve)?;

        self.attachment()
            .map(|attach| {
                attach.verify().map(|_| {
                    schema.attach_contract_notification(
                        contract_tx_hash,
                        tx_hash,
                        attach.metadata(),
                    )
                })
            })
            .transpose()?;

        for ownership in contract.conditions().objects() {
            schema.set_unpublished_contract(&ownership.object().id(), contract_tx_hash);
        }
        schema.update_contract(contract_tx_hash, contract);

        Ok(())
    }
}

impl Transaction for RejectContract {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        self.attachment()
            .map(|attach| attach.verify())
            .transpose()
            .is_ok()
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let old_contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        let status = ContractStatus::try_from(old_contract.state())?;
        if status == ContractStatus::Registering || status == ContractStatus::AwaitingUserAction {
            self.attachment()
                .map(|attach| {
                    attach.verify().map(|_| {
                        schema.attach_contract_notification(
                            contract_tx_hash,
                            tx_hash,
                            attach.metadata(),
                        )
                    })
                })
                .transpose()?;
        };
        let new_contract = old_contract.apply(Action::Reject)?;
        for ownership in new_contract.conditions().objects() {
            schema.set_unpublished_contract(&ownership.object().id(), contract_tx_hash);
        }
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
        schema.add_contracts_contacts_mut(
            contract_tx_hash,
            self.contract_correspondence(),
            self.objects_correspondence(),
        );
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

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

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for PublishBids {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        !self.bids().is_empty()
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
            Error::lot_is_not_verified(lot_id).ok()?
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
            requestor,
            lot.price(),
            lot.sale_type(),
            lot.opening_time(),
            self.new_expiration_date(),
        );

        // schema.remove_lot(lot_id);
        schema.update_lot(*lot_id, lot, conditions);
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

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
        if !state.is_verified() {
            Error::lot_is_not_verified(lot_id).ok()?
        }

        let newstate = state.set_status_executed();
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

    fn execute(&self, fork: &mut Fork, tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let requestor = self.requestor();
        let old_contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;

        if old_contract.is_undefined() {
            Error::contract_is_undefined(contract_tx_hash).ok()?;
        }

        let new_contract = old_contract.apply(Action::Sign(requestor.clone()))?;

        let deed_file = schema
            .contract_deed(contract_tx_hash)
            .ok_or(Error::deed_file_not_found(contract_tx_hash))?;
        let deed_attachment: Attachment = schema.get_attachment(deed_file.tx_hash())?;
        let deed_data = deed_attachment.data();
        let deed_sign = self.deed_sign();
        member_matches_sign(&requestor, &deed_sign)?;
        deed_sign.verify_data(deed_data)?;

        let application_file = schema
            .contract_application(contract_tx_hash)
            .ok_or(Error::deed_file_not_found(contract_tx_hash))?;
        let application_attachment = schema.get_attachment(application_file.tx_hash())?;
        let application_data = application_attachment.data();
        let application_sign = self.application_sign();
        member_matches_sign(&requestor, &application_sign)?;
        application_sign.verify_data(application_data)?;

        // The row `let new_contract = old_contract.apply(Action::Sign(self.requestor()))?;` guarantee that requestor buyer or seller.
        match requestor {
            buyer if new_contract.is_buyer(&buyer) => {
                schema.add_sign_contract_tx(
                    deed_file.tx_hash(),
                    buyer.clone(),
                    BuyerSeller::Buyer,
                    tx_hash,
                );
                schema.add_sign_contract_tx(
                    application_file.tx_hash(),
                    buyer,
                    BuyerSeller::Buyer,
                    tx_hash,
                );
            }
            seller if new_contract.is_seller(&seller) => {
                schema.add_sign_contract_tx(
                    deed_file.tx_hash(),
                    seller.clone(),
                    BuyerSeller::Seller,
                    tx_hash,
                );
                schema.add_sign_contract_tx(
                    application_file.tx_hash(),
                    seller,
                    BuyerSeller::Seller,
                    tx_hash,
                );
            }
            _ => Error::no_permissions().ok()?,
        }

        schema.update_contract(contract_tx_hash, new_contract);
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for ContractSubmitChecks {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let mut contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        let status = ContractStatus::try_from(contract.state())?;
        match self.is_undef() {
            true => {
                if !contract.is_undefined() {
                    Error::bad_state("contract should be 'undefined'").ok()?;
                };
                contract = contract.set_undefined(false);
                match status {
                    ContractStatus::Draft(_) | ContractStatus::Confirmed(_) => {
                        if self.checks().iter().any(|v| v.result().is_error()) {
                            contract = contract.apply(Action::MakeDraft)?;
                        }
                    }
                    ContractStatus::Signed => {
                        if self.checks().iter().any(|v| v.result().is_error()) {
                            contract = contract.apply(Action::Reject)?;
                            for ownership in contract.conditions().objects() {
                                schema.set_unpublished_contract(
                                    &ownership.object().id(),
                                    contract_tx_hash,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            false => match status {
                ContractStatus::New => {
                    schema
                        .contract_deed(contract_tx_hash)
                        .ok_or(Error::deed_file_not_found(contract_tx_hash))?;
                    schema
                        .contract_application(contract_tx_hash)
                        .ok_or(Error::application_file_not_found(contract_tx_hash))?;

                    contract = contract.apply(Action::MakeDraft)?;
                }

                _ => {}
            },
        }
        if let Some(ref_numb) = self.reference_number() {
            schema.contract_add_reference_number(contract_tx_hash, ref_numb);
        }

        schema.update_contract(contract_tx_hash, contract);
        schema.apply_checks(contract_tx_hash, self.checks());
        Ok(())
    }
}

impl Transaction for LotSubmitChecks {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_tx_hash = self.lot_tx_hash();
        let state = schema
            .lot_states()
            .get(lot_tx_hash)
            .ok_or_else(|| Error::no_lot(lot_tx_hash))?;

        match self.is_undef() {
            true => {
                if !state.undefined() {
                    Error::bad_state("lot status should be 'undefined'").ok()?
                }
                let mut new_state = state.set_undefined(false);

                if (new_state.is_new() || new_state.is_verified())
                    && self.checks().iter().any(|v| v.result().is_error())
                {
                    let conditions = schema
                        .lot_conditions()
                        .get(lot_tx_hash)
                        .ok_or_else(|| Error::no_lot(lot_tx_hash))?;

                    for object in conditions.objects() {
                        let obj_id = &object.object().id();
                        schema.set_unpublished(obj_id, lot_tx_hash);
                    }
                    new_state = new_state.set_status_rejected();
                };
                schema.set_lot_state(lot_tx_hash, new_state);
            }
            false => {
                if state.is_new() {
                    let new_state = if self.checks().iter().any(|v| v.result().is_error()) {
                        let conditions = schema
                            .lot_conditions()
                            .get(lot_tx_hash)
                            .ok_or_else(|| Error::no_lot(lot_tx_hash))?;

                        for object in conditions.objects() {
                            let obj_id = &object.object().id();
                            schema.set_unpublished(obj_id, lot_tx_hash);
                        }
                        state.set_status_rejected()
                    } else {
                        state.set_status_verified()
                    };
                    schema.set_lot_state(lot_tx_hash, new_state);
                }
            }
        }

        if let Some(ref_numb) = self.reference_number() {
            schema.lot_add_reference_number(lot_tx_hash, ref_numb);
        }

        schema.apply_checks(lot_tx_hash, self.checks());
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

        schema.contract_add_reference_number(contract_tx_hash, self.reference_number().to_string());
        Ok(())
    }
}

impl Transaction for TaxRequest {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn participants(&self) -> Vec<PublicKey> {
        self.share()
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let schema = Schema::new(fork);
        let contract_tx_hash = self.contract_tx_hash();
        let contract = schema
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        if !contract.is_signed()? {
            let status = ContractStatus::try_from(contract.state())?;
            Error::bad_contract_state(status, "TaxRequest").ok()?;
        }

        let requestor = self.requestor();

        if !contract.is_member(&requestor) {
            Error::no_permissions().ok()?
        }
        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for TaxContractCalculation {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
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

        if !contract.check_modifiable()? {
            let status = ContractStatus::try_from(contract.state())?;
            Error::bad_contract_state(status, "adding calculation").ok()?;
        };
        //TODO Желательно сделать проверку на количество расчетов, должен быть 1 или 2 в зависимости от списка ОИС

        if let Some(ref_numb) = self.reference_number() {
            schema.contract_add_reference_number(contract_tx_hash, ref_numb);
        }

        schema.add_contract_calculations(contract_tx_hash, self.calculations());
        Ok(())
    }
}

impl Transaction for TaxLotCalculation {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _tx_hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let lot_tx_hash = self.lot_tx_hash();
        let calculations = self.calculations();
        let state = schema
            .lot_states()
            .get(lot_tx_hash)
            .ok_or_else(|| Error::no_lot(lot_tx_hash))?;

        //TODO Желательно сделать проверку на количество расчетов, должен быть 1 или 2 в зависимости от списка ОИС

        if state.is_new() || state.is_rejected() {
            Error::bad_state("lot status should be 'verified' before adding calculation").ok()?
        }

        if let Some(ref_numb) = self.reference_number() {
            schema.lot_add_reference_number(lot_tx_hash, ref_numb);
        }

        schema.add_lot_calculations(lot_tx_hash, calculations);
        Ok(())
    }
}

impl Transaction for TaxWithPaymentDetails {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
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

        if !contract.is_signed()? {
            let status = ContractStatus::try_from(contract.state())?;
            Error::bad_contract_state(status, "TaxWithPaymentDetails").ok()?;
        }

        //TODO Желательно сделать проверку на количество расчетов, должен быть 1 или 2 в зависимости от списка ОИС

        let mut payment_details =
            PaymentDetailsWrapper::from(schema.get_contract_payment_details(contract_tx_hash))
                .get_all_paid();
        payment_details.extend(self.payment_details());
        schema.add_contract_payment_details(contract_tx_hash, payment_details);

        if contract.is_signed()? {
            let new_contract = contract.apply(Action::ReadyForRegistering)?;
            schema.update_contract(contract_tx_hash, new_contract);
        }

        Ok(())
    }
}

impl Transaction for TaxStatus {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
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

        if !contract.is_signed()? {
            let status = ContractStatus::try_from(contract.state())?;
            Error::bad_contract_state(status, "TaxWithPaymentDetails").ok()?;
        }

        //TODO Желательно сделать проверку на количество расчетов, должен быть 1 или 2 в зависимости от списка ОИС

        schema.change_contract_payment_status(
            contract_tx_hash,
            self.payment_id(),
            self.status().try_into()?,
        )?;
        Ok(())
    }
}

impl Transaction for ContractUndefined {
    fn verify(&self, _certificates: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        let contract_id = self.contract_tx_hash();
        let contract = schema
            .contracts()
            .get(contract_id)
            .ok_or_else(|| Error::no_contract(contract_id))?;

        if contract.is_finished()? {
            Error::bad_state("The contract has already been completed").ok()?
        };

        let new_contract = match self.admit() {
            true => contract.set_undefined(false),
            false => {
                for ownership in contract.conditions().objects() {
                    schema.set_unpublished_contract(&ownership.object().id(), contract_id);
                }
                contract.apply(Action::Reject)?
            }
        };

        schema.update_contract(contract_id, new_contract);
        Ok(())
    }
}

impl Transaction for MemberToken {
    fn verify(&self, _certificates: &HashMap<PublicKey, Certificate>) -> bool {
        true
    }

    fn execute(&self, fork: &mut Fork, _hash: &Hash, _executor: &PublicKey) -> ExecutionResult {
        let mut schema = Schema::new(fork);
        schema.put_member_token(
            &self.member(),
            self.token().to_owned(),
            self.oid().to_owned(),
        );
        Ok(())
    }

    fn pre_execute(
        &self,
        _snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let member = self.member();

        let is_success = EsiaAuth::validate(&member, self.token(), self.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for ContractConfirmCreate {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
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

        let requestor_id = &self.requestor();

        if !contract.is_member(requestor_id) {
            Error::no_permissions().ok()?
        };

        let contract = contract.apply(Action::Confirm(requestor_id.to_owned()))?;
        schema.update_contract(self.contract_tx_hash(), contract);

        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.requestor();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for ContractUnconfirmCreate {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
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

        let member_id = &self.member();

        if !contract.is_member(member_id) {
            Error::no_permissions().ok()?
        };

        let contract = contract.apply(Action::Unconfirm(self.member()))?;
        schema.update_contract(self.contract_tx_hash(), contract);

        Ok(())
    }

    fn pre_execute(
        &self,
        snapshot: &dyn Snapshot,
        _hash: &Hash,
        _executor: &PublicKey,
    ) -> PreExecutionResult {
        let schema = Schema::new(snapshot);
        let member = self.member();
        let token = schema
            .member_token(&member)
            .ok_or_else(|| Error::no_member_token())?;

        let is_success = EsiaAuth::validate(&member, token.token(), token.oid())?;

        if !is_success {
            Error::esia_invalid_member(&member).ok()?
        }

        Ok(())
    }
}

impl Transaction for ContractNew {
    fn verify(&self, _certs: &HashMap<PublicKey, Certificate>) -> bool {
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

        let contract = contract.apply(Action::New)?;
        schema.update_contract(self.contract_tx_hash(), contract);

        Ok(())
    }
}
