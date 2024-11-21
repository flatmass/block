use std::collections::HashMap;

use blockp_core::blockchain::Schema as CoreSchema;
use blockp_core::crypto::{Hash, PublicKey};
use blockp_core::messages::RawMessage;
use blockp_core::storage::{Fork, ListIndex, MapIndex, ProofListIndex, Snapshot, ValueSetIndex};

use crate::data::attachment::{
    Attachment, AttachmentMetadata, AttachmentMetadataWithHash, DocumentId,
};
use crate::data::conditions::{Check, CheckResult, Conditions};
use crate::data::contract::{
    BuyerSeller, Contract, ContractId, ContractSign, CorrespondenceContacts, State,
};
use crate::data::lot::{Bid, Lot, LotId, LotState};
use crate::data::member::{MemberEsiaToken, MemberId, MemberIdentity};
use crate::data::object::{Change, ObjectId, ObjectIdentity};
use crate::data::ownership::{OwnershipUnstructured, Rights};
use crate::data::payment::{Calculation, PaymentDetail, PaymentDetailsWrapper, PaymentStatus};
use crate::error::{Error, Result};
use crate::transactions::{get_private_tx, OwnershipTransactions};

const CONTRACT_CALCULATIONS_INDEX: &str = "fips.contract.calculations";
const LOT_CALCULATIONS_INDEX: &str = "fips.lot.calculations";

#[derive(Debug)]
pub struct Schema<T> {
    view: T,
}

impl<T> AsMut<T> for Schema<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.view
    }
}

impl<T> Schema<T>
where
    T: AsRef<dyn Snapshot>,
{
    pub fn new(view: T) -> Self {
        Schema { view }
    }

    pub fn objects(&self) -> MapIndex<&T, ObjectId, String> {
        MapIndex::new("fips.objects", &self.view)
    }

    pub fn objects_list(&self) -> ListIndex<&T, ObjectId> {
        ListIndex::new("fips.objects.list", &self.view)
    }

    pub fn objects_identity(&self) -> MapIndex<&T, ObjectId, ObjectIdentity> {
        MapIndex::new("fips.objects_identity", &self.view)
    }

    pub fn rightholders(&self, object_id: &ObjectId) -> MapIndex<&T, MemberIdentity, Rights> {
        MapIndex::new_in_family("fips.rightholders", object_id, &self.view)
    }

    pub fn object_history(&self, object_id: &ObjectId) -> ProofListIndex<&T, Change> {
        ProofListIndex::new_in_family("fips.object_history", object_id, &self.view)
    }

    pub fn ownership(&self, member_id: &MemberId) -> ValueSetIndex<&T, ObjectIdentity> {
        ValueSetIndex::new_in_family("fips.ownership", member_id, &self.view)
    }

    pub fn ownership_unstructured(
        &self,
        object_id: &ObjectId,
    ) -> ListIndex<&T, OwnershipUnstructured> {
        ListIndex::new_in_family("fips.ownership_unstructured", object_id, &self.view)
    }

    pub fn object_publications(&self, object_id: &ObjectId) -> MapIndex<&T, LotId, ()> {
        MapIndex::new_in_family("fips.publications", object_id, &self.view)
    }

    pub fn object_publications_contract(
        &self,
        object_id: &ObjectId,
    ) -> MapIndex<&T, ContractId, ()> {
        MapIndex::new_in_family("fips.publications.contract", object_id, &self.view)
    }

    /*pub fn is_published(&self, object_id: &ObjectId) -> bool {
        self.object_publications(object_id).iter().next().is_some()
    }*/

    pub fn lots(&self) -> MapIndex<&T, LotId, Lot> {
        MapIndex::new("fips.lots", &self.view)
    }

    pub fn lots_list(&self) -> ListIndex<&T, LotId> {
        ListIndex::new("fips.lots.list", &self.view)
    }

    pub fn lot_conditions(&self) -> MapIndex<&T, LotId, Conditions> {
        MapIndex::new("fips.lot_conditions", &self.view)
    }

    pub fn lot_states(&self) -> MapIndex<&T, LotId, LotState> {
        MapIndex::new("fips.lot_states", &self.view)
    }

    pub fn member_lots(&self, member_id: &MemberId) -> ValueSetIndex<&T, LotId> {
        ValueSetIndex::new_in_family("fips.member_lots", member_id, &self.view)
    }

    pub fn bids(&self, lot_id: &LotId) -> ListIndex<&T, Bid> {
        ListIndex::new_in_family("fips.bids", lot_id, &self.view)
    }

    pub fn bid_history(&self, lot_id: &LotId) -> ListIndex<&T, Hash> {
        ListIndex::new_in_family("fips.bid_history", lot_id, &self.view)
    }

    pub fn contracts(&self) -> MapIndex<&T, ContractId, Contract> {
        MapIndex::new("fips.contracts", &self.view)
    }

    pub fn correspondence_contacts(&self) -> MapIndex<&T, ContractId, CorrespondenceContacts> {
        MapIndex::new("fips.contracts.correspondence_contacts", &self.view)
    }

    pub fn checks(&self, id: &Hash) -> MapIndex<&T, u16, CheckResult> {
        MapIndex::new_in_family("fips.checks", id, &self.view)
    }

    pub fn member_contracts(&self, member_id: &MemberId) -> MapIndex<&T, ContractId, ()> {
        MapIndex::new_in_family("fips.member_contracts", member_id, &self.view)
    }

    // pub fn attachments(&self, member_id: &MemberId) -> MapIndex<&T, DocumentId, Hash> {
    //     MapIndex::new_in_family("fips.attachments", member_id, &self.view)
    // }

    pub fn member_token(&self, member_id: &MemberIdentity) -> Option<MemberEsiaToken> {
        let index: MapIndex<&T, MemberIdentity, MemberEsiaToken> =
            MapIndex::new("fips.esia.member.token", &self.view);
        index.get(member_id)
    }

    pub fn contract_files(
        &self,
        contract_id: &ContractId,
    ) -> MapIndex<&T, DocumentId, AttachmentMetadata> {
        MapIndex::new_in_family("fips.contract_files", contract_id, &self.view)
    }

    pub fn contract_notifications(
        &self,
        contract_id: &ContractId,
    ) -> MapIndex<&T, DocumentId, AttachmentMetadata> {
        MapIndex::new_in_family("fips.contract_notifications", contract_id, &self.view)
    }

    pub fn contract_deed(&self, contract_id: &ContractId) -> Option<AttachmentMetadataWithHash> {
        let storage: MapIndex<&T, ContractId, AttachmentMetadataWithHash> =
            MapIndex::new("fips.contract_files.deed", &self.view);
        storage.get(contract_id)
    }

    pub fn contract_application(
        &self,
        contract_id: &ContractId,
    ) -> Option<AttachmentMetadataWithHash> {
        let storage: MapIndex<&T, ContractId, AttachmentMetadataWithHash> =
            MapIndex::new("fips.contract_files.application", &self.view);
        storage.get(contract_id)
    }

    pub fn get_attachment(&self, document_id: &DocumentId) -> Result<Attachment> {
        let doc_tx = get_private_tx(&self, document_id)?;
        match doc_tx {
            OwnershipTransactions::AttachContractMainFile(tx) => Ok(tx.file()),
            OwnershipTransactions::AttachContractOtherFile(tx) => Ok(tx.file()),
            OwnershipTransactions::ApproveContract(tx) => Ok(tx
                .attachment()
                .ok_or_else(|| Error::no_attachment(document_id))?),
            OwnershipTransactions::RejectContract(tx) => Ok(tx
                .attachment()
                .ok_or_else(|| Error::no_attachment(document_id))?),
            _ => Error::unexpected_tx_type(document_id).ok(),
        }
    }

    fn deprecated_sign_contract_tx(&self) -> MapIndex<&T, DocumentId, Hash> {
        MapIndex::new("fips.attachment_signs", &self.view)
    }

    fn sign_contract_tx(&self) -> MapIndex<&T, DocumentId, ContractSign> {
        MapIndex::new("fips.attachment_signs_v2", &self.view)
    }

    // It will contain hash of SignContract transaction for deed and application document if contract
    pub fn deprecated_get_sign_contract_tx(&self, document_id: &DocumentId) -> Option<Hash> {
        self.deprecated_sign_contract_tx().get(document_id)
    }

    pub fn participants(&self, member_id: &MemberId) -> ListIndex<&T, String> {
        ListIndex::new_in_family("fips.participants", member_id, &self.view)
    }

    // It will contain hash of SignContract transaction for deed and application document if contract
    pub fn get_sign_contract_tx(&self, document_id: &DocumentId) -> Option<ContractSign> {
        self.sign_contract_tx().get(document_id)
    }

    // core as a dependency
    pub fn core_transactions(&self) -> MapIndex<&T, Hash, RawMessage> {
        MapIndex::new("core.transactions", &self.view)
    }

    fn contract_calculations(&self) -> MapIndex<&T, ContractId, PaymentDetailsWrapper> {
        MapIndex::new(CONTRACT_CALCULATIONS_INDEX, &self.view)
    }

    pub fn get_contract_calculations(&self, contract_tx_hash: &ContractId) -> Vec<Calculation> {
        self.contract_calculations()
            .get(contract_tx_hash)
            .unwrap_or_else(|| PaymentDetailsWrapper::new(vec![]))
            .payment_details()
            .into_iter()
            .map(|v| v.calculation())
            .collect()
    }

    pub fn get_contract_payment_details(
        &self,
        contract_tx_hash: &ContractId,
    ) -> Vec<PaymentDetail> {
        self.contract_calculations()
            .get(contract_tx_hash)
            .unwrap_or_else(|| PaymentDetailsWrapper::new(vec![]))
            .payment_details()
    }

    fn lot_calculations(&self) -> MapIndex<&T, ContractId, PaymentDetailsWrapper> {
        MapIndex::new(LOT_CALCULATIONS_INDEX, &self.view)
    }

    pub fn get_lot_calculations(&self, lot_tx_hash: &LotId) -> Vec<Calculation> {
        self.lot_calculations()
            .get(lot_tx_hash)
            .unwrap_or_else(|| PaymentDetailsWrapper::new(vec![]))
            .payment_details()
            .into_iter()
            .map(|v| v.calculation())
            .collect()
    }

    pub fn is_all_payments_paid(&self, contract_tx_hash: &ContractId) -> bool {
        self.get_contract_payment_details(contract_tx_hash)
            .iter()
            .all(|x| match x.is_paid() {
                Ok(x) => x,
                Err(_) => false,
            })
    }

    // TODO time service as a dependency
    /*pub fn time(&self) -> Entry<&T, DateTime<Utc>> {
        Entry::new("exonum_time.time", &self.view)
    }*/

    pub fn is_owner(&self, member_id: &MemberIdentity, obj_id: &ObjectId) -> bool {
        self.rightholders(obj_id)
            .get(member_id)
            .map(|rights| rights.is_owner())
            .unwrap_or_default()
    }

    pub fn find_owner(&self, obj_id: &ObjectId) -> Option<MemberIdentity> {
        self.rightholders(obj_id)
            .iter()
            .find(|(_, rights)| rights.is_owner())
            .map(|(uid, _)| uid)
    }

    pub fn is_validator(&self, node_id: &PublicKey) -> bool {
        CoreSchema::new(&self.view)
            .actual_configuration()
            .validator_keys
            .iter()
            .any(|v| v.consensus_key.eq(node_id))
    }

    pub fn rights(&self, member_id: &MemberIdentity, obj_id: &ObjectId) -> Option<Rights> {
        self.rightholders(obj_id).get(member_id)
    }

    pub fn core_private_data(&self) -> MapIndex<&T, Hash, RawMessage> {
        MapIndex::new("core.private_data", &self.view)
    }

    pub fn contract_reference_number(&self, contract_tx_hash: &ContractId) -> Option<String> {
        let storage: MapIndex<&T, ContractId, String> =
            MapIndex::new("fips.contracts.reference_number", &self.view);
        storage.get(contract_tx_hash)
    }

    pub fn lot_reference_number(&self, lot_tx_hash: &LotId) -> Option<String> {
        let storage: MapIndex<&T, LotId, String> =
            MapIndex::new("fips.lots.reference_number", &self.view);
        storage.get(lot_tx_hash)
    }

    pub fn state_hash(&self) -> Vec<Hash> {
        // TODO
        vec![]
    }

    pub fn get_contract_share(&self, contract_tx_hash: &ContractId) -> Result<Vec<PublicKey>> {
        let contract = self
            .contracts()
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_contract(contract_tx_hash))?;
        [&contract.seller(), &contract.buyer()]
            .iter()
            .flat_map(|p| self.participants(&p.id()).iter().collect::<Vec<String>>())
            .map(|s| {
                PublicKey::from_slice(s.as_bytes())
                    .ok_or_else(|| Error::bad_stored_member(s.as_str()))
            })
            .collect::<Result<Vec<PublicKey>>>()
    }

    pub fn check_result(&self, tx_hash: &Hash) -> Result<()> {
        let errors = self
            .checks(tx_hash)
            .iter()
            .filter(|(_, res)| res.is_error())
            .map(|(k, res)| Check::new(k, res))
            .collect::<Vec<Check>>();

        if errors.len() == 0 {
            Ok(())
        } else {
            Error::check_failed(errors).ok()?
        }
    }

    pub fn lots_to_invalidate(&self, object_id: &ObjectId) -> Vec<(LotId, LotState)> {
        self.object_publications(object_id)
            .iter()
            .filter_map(|(lot_id, _)| {
                let state = self.lot_states().get(&lot_id).unwrap();
                // TODO need to check conditions
                if !state.is_closed() {
                    Some((lot_id, state.set_undefined(true)))
                } else {
                    None
                }
            })
            .collect::<Vec<(LotId, LotState)>>()
    }

    pub fn contracts_to_invalidate(&self, object_id: &ObjectId) -> Vec<(ContractId, Contract)> {
        self.object_publications_contract(object_id)
            .iter()
            .filter_map(|(contract_id, _)| {
                // TODO! need to remove unwrap()
                let contract = self.contracts().get(&contract_id).unwrap();
                let state = State::from_bits(contract.state()).unwrap();
                if state == State::SIGNED
                    || state == State::REFUSED
                    || state == State::REJECTED
                    || state == State::APPROVED
                    || state == State::REGISTERING
                    || state == State::AWAITING_USER_ACTION
                {
                    None
                } else {
                    Some((contract_id, contract.set_undefined(true)))
                }
            })
            .collect::<Vec<(ContractId, Contract)>>()
    }
}

impl<'a> Schema<&'a mut Fork> {
    fn objects_mut(&mut self) -> MapIndex<&mut Fork, ObjectId, String> {
        MapIndex::new("fips.objects", &mut self.view)
    }

    pub fn objects_list_mut(&mut self) -> ListIndex<&mut Fork, ObjectId> {
        ListIndex::new("fips.objects.list", &mut self.view)
    }

    fn objects_identity_mut(&mut self) -> MapIndex<&mut Fork, ObjectId, ObjectIdentity> {
        MapIndex::new("fips.objects_identity", &mut self.view)
    }

    fn rightholders_mut(
        &mut self,
        object_id: &ObjectId,
    ) -> MapIndex<&mut Fork, MemberIdentity, Rights> {
        MapIndex::new_in_family("fips.rightholders", object_id, &mut self.view)
    }

    fn object_history_mut(&mut self, object_id: &ObjectId) -> ProofListIndex<&mut Fork, Change> {
        ProofListIndex::new_in_family("fips.object_history", object_id, &mut self.view)
    }

    fn ownership_mut(&mut self, member_id: &MemberId) -> ValueSetIndex<&mut Fork, ObjectIdentity> {
        ValueSetIndex::new_in_family("fips.ownership", member_id, &mut self.view)
    }

    fn ownership_unstructured_mut(
        &mut self,
        object_id: &ObjectId,
    ) -> ListIndex<&mut Fork, OwnershipUnstructured> {
        ListIndex::new_in_family("fips.ownership_unstructured", object_id, &mut self.view)
    }

    fn object_publications_mut(&mut self, object_id: &ObjectId) -> MapIndex<&mut Fork, LotId, ()> {
        MapIndex::new_in_family("fips.publications", object_id, &mut self.view)
    }

    fn object_publications_contract_mut(
        &mut self,
        object_id: &ObjectId,
    ) -> MapIndex<&mut Fork, ContractId, ()> {
        MapIndex::new_in_family("fips.publications.contract", object_id, &mut self.view)
    }

    fn lots_mut(&mut self) -> MapIndex<&mut Fork, LotId, Lot> {
        MapIndex::new("fips.lots", &mut self.view)
    }

    fn lots_list_mut(&mut self) -> ListIndex<&mut Fork, LotId> {
        ListIndex::new("fips.lots.list", &mut self.view)
    }

    fn lot_conditions_mut(&mut self) -> MapIndex<&mut Fork, LotId, Conditions> {
        MapIndex::new("fips.lot_conditions", &mut self.view)
    }

    fn lot_states_mut(&mut self) -> MapIndex<&mut Fork, LotId, LotState> {
        MapIndex::new("fips.lot_states", &mut self.view)
    }

    fn member_lots_mut(&mut self, member_id: &MemberId) -> ValueSetIndex<&mut Fork, LotId> {
        ValueSetIndex::new_in_family("fips.member_lots", member_id, &mut self.view)
    }

    fn bids_mut(&mut self, lot_id: &LotId) -> ListIndex<&mut Fork, Bid> {
        ListIndex::new_in_family("fips.bids", lot_id, &mut self.view)
    }

    fn bid_history_mut(&mut self, lot_id: &LotId) -> ListIndex<&mut Fork, Hash> {
        ListIndex::new_in_family("fips.bid_history", lot_id, &mut self.view)
    }

    fn contracts_mut(&mut self) -> MapIndex<&mut Fork, ContractId, Contract> {
        MapIndex::new("fips.contracts", &mut self.view)
    }

    fn correspondence_contacts_mut(
        &mut self,
    ) -> MapIndex<&mut Fork, ContractId, CorrespondenceContacts> {
        MapIndex::new("fips.contracts.correspondence_contacts", &mut self.view)
    }

    pub fn put_member_token(&mut self, member_id: &MemberIdentity, token: String, oid: String) {
        let mut index: MapIndex<&mut Fork, MemberIdentity, MemberEsiaToken> =
            MapIndex::new("fips.esia.member.token", &mut self.view);
        index.put(member_id, MemberEsiaToken::new(&token, &oid))
    }

    pub fn add_contracts_contacts_mut(
        &mut self,
        contract_id: &ContractId,
        contract_correspondence: Option<String>,
        objects_correspondence: Option<String>,
    ) {
        match (&contract_correspondence, &objects_correspondence) {
            (Some(_), Some(_)) => self.correspondence_contacts_mut().put(
                contract_id,
                CorrespondenceContacts::new(contract_correspondence, objects_correspondence),
            ),
            (None, None) => return,
            (None, Some(_)) => {
                let contract_correspondence = self
                    .correspondence_contacts()
                    .get(contract_id)
                    .and_then(|v| v.contract_correspondence());
                self.correspondence_contacts_mut().put(
                    contract_id,
                    CorrespondenceContacts::new(contract_correspondence, objects_correspondence),
                )
            }
            (Some(_), None) => {
                let objects_correspondence = self
                    .correspondence_contacts()
                    .get(contract_id)
                    .and_then(|v| v.objects_correspondence());
                self.correspondence_contacts_mut().put(
                    contract_id,
                    CorrespondenceContacts::new(contract_correspondence, objects_correspondence),
                )
            }
        }
    }

    fn checks_mut(&mut self, id: &Hash) -> MapIndex<&mut Fork, u16, CheckResult> {
        MapIndex::new_in_family("fips.checks", id, &mut self.view)
    }

    fn member_contracts_mut(
        &mut self,
        member_id: &MemberId,
    ) -> MapIndex<&mut Fork, ContractId, ()> {
        MapIndex::new_in_family("fips.member_contracts", member_id, &mut self.view)
    }

    // fn attachments_mut(&mut self, member_id: &MemberId) -> MapIndex<&mut Fork, DocumentId, Hash> {
    //     MapIndex::new_in_family("fips.attachments", member_id, &mut self.view)
    // }

    fn contract_files_mut(
        &mut self,
        contract_id: &ContractId,
    ) -> MapIndex<&mut Fork, DocumentId, AttachmentMetadata> {
        MapIndex::new_in_family("fips.contract_files", contract_id, &mut self.view)
    }

    fn contract_notifications_mut(
        &mut self,
        contract_id: &ContractId,
    ) -> MapIndex<&mut Fork, DocumentId, AttachmentMetadata> {
        MapIndex::new_in_family("fips.contract_notifications", contract_id, &mut self.view)
    }

    fn contract_deed_mut(&mut self) -> MapIndex<&mut Fork, ContractId, AttachmentMetadataWithHash> {
        MapIndex::new("fips.contract_files.deed", &mut self.view)
    }

    fn contract_application_mut(
        &mut self,
    ) -> MapIndex<&mut Fork, ContractId, AttachmentMetadataWithHash> {
        MapIndex::new("fips.contract_files.application", &mut self.view)
    }

    fn deprecated_sign_contract_tx_mut(&mut self) -> MapIndex<&mut Fork, DocumentId, Hash> {
        MapIndex::new("fips.attachment_signs", &mut self.view)
    }

    fn sign_contract_tx_mut(&mut self) -> MapIndex<&mut Fork, DocumentId, ContractSign> {
        MapIndex::new("fips.attachment_signs_v2", &mut self.view)
    }

    fn participants_mut(&mut self, member_id: &MemberId) -> ListIndex<&mut Fork, String> {
        ListIndex::new_in_family("fips.participants", member_id, &mut self.view)
    }

    fn contract_reference_number_mut(&mut self) -> MapIndex<&mut Fork, ContractId, String> {
        MapIndex::new("fips.contracts.reference_number", &mut self.view)
    }

    pub fn contract_add_reference_number(&mut self, contract_id: &ContractId, ref_number: String) {
        self.contract_reference_number_mut()
            .put(contract_id, ref_number);
    }

    fn lot_reference_number_mut(&mut self) -> MapIndex<&mut Fork, LotId, String> {
        MapIndex::new("fips.lots.reference_number", &mut self.view)
    }

    pub fn lot_add_reference_number(&mut self, lot_id: &LotId, ref_number: String) {
        self.lot_reference_number_mut().put(lot_id, ref_number);
    }

    pub fn update_object_history(&mut self, object_id: &ObjectId, change: Change) -> Hash {
        let mut history = self.object_history_mut(object_id);
        history.push(change);
        history.merkle_root()
    }

    pub fn invalidate_published_lots(&mut self, object_id: &ObjectId) {
        let states = self.lots_to_invalidate(object_id);
        for (lot_id, state) in states {
            self.set_lot_state(&lot_id, state);
        }
    }

    pub fn invalidate_published_contracts(&mut self, object_id: &ObjectId) {
        let states = self.contracts_to_invalidate(object_id);
        for (contract_id, contract) in states {
            self.update_contract(&contract_id, contract);
        }
    }

    pub fn update_rights(
        &mut self,
        object: &ObjectIdentity,
        rights: HashMap<MemberIdentity, Rights>,
    ) {
        let object_id = &object.id();
        let to_remove = self
            .rightholders(object_id)
            .keys()
            .filter(|uid| !rights.contains_key(uid))
            .collect::<Vec<MemberIdentity>>();
        for uid in rights.keys() {
            self.ownership_mut(&uid.id()).insert(object.clone());
        }
        for uid in to_remove.iter() {
            self.ownership_mut(&uid.id()).remove(&object);
        }
        let mut rightholders = self.rightholders_mut(object_id);
        for uid in to_remove.iter() {
            rightholders.remove(&uid);
        }
        for (uid, rights) in rights.into_iter() {
            rightholders.put(&uid, rights);
        }
    }

    pub fn update_unstructured_ownership(
        &mut self,
        object: &ObjectIdentity,
        ownership: Vec<OwnershipUnstructured>,
    ) {
        let object_id = &object.id();
        self.ownership_unstructured_mut(object_id).clear();
        self.ownership_unstructured_mut(object_id).extend(ownership);
    }

    pub fn update_object_data(
        &mut self,
        obj_id: &ObjectId,
        data: &str,
        tx_hash: &Hash,
        object: ObjectIdentity,
    ) {
        let change = Change::new(tx_hash);
        self.objects_mut().put(obj_id, data.to_string());
        self.objects_list_mut().push(obj_id.clone());
        self.objects_identity_mut().put(obj_id, object);
        self.update_object_history(obj_id, change);
    }

    pub fn set_published(&mut self, object_id: &ObjectId, lot_id: &LotId) {
        self.object_publications_mut(object_id).put(lot_id, ())
    }

    pub fn set_unpublished(&mut self, object_id: &ObjectId, lot_id: &LotId) {
        self.object_publications_mut(object_id).remove(lot_id)
    }

    pub fn set_published_contract(&mut self, object_id: &ObjectId, contract_id: &ContractId) {
        self.object_publications_contract_mut(object_id)
            .put(contract_id, ())
    }

    pub fn set_unpublished_contract(&mut self, object_id: &ObjectId, contract_id: &ContractId) {
        self.object_publications_contract_mut(object_id)
            .remove(contract_id)
    }

    pub fn add_lot(&mut self, lot_id: LotId, lot: Lot, conditions: Conditions) {
        self.lots_mut().put(&lot_id, lot);
        self.lot_conditions_mut().put(&lot_id, conditions);
        self.lots_list_mut().push(lot_id);
    }

    pub fn update_lot(&mut self, lot_id: LotId, lot: Lot, conditions: Conditions) {
        self.lots_mut().put(&lot_id, lot);
        self.lot_conditions_mut().put(&lot_id, conditions);
    }

    pub fn _remove_lot(&mut self, lot_id: &LotId) {
        self.lots_mut().remove(lot_id);
        self.lot_conditions_mut().remove(lot_id);
    }

    pub fn remove_lot_data(&mut self, lot_id: &LotId) {
        self.remove_lot_calculations(lot_id);
    }

    pub fn add_member_lot(&mut self, member_id: &MemberId, lot_id: &LotId) {
        self.member_lots_mut(member_id).insert(*lot_id);
    }

    pub fn remove_member_lot(&mut self, member_id: &MemberId, lot_id: &LotId) {
        self.member_lots_mut(member_id).remove(lot_id);
    }

    pub fn set_lot_state(&mut self, lot_id: &LotId, state: LotState) {
        self.lot_states_mut().put(lot_id, state);
    }

    pub fn remove_lot_state(&mut self, lot_id: &LotId) {
        self.lot_states_mut().remove(lot_id);
    }

    pub fn add_bid(&mut self, lot_id: &LotId, bid: Bid) {
        self.bids_mut(lot_id).push(bid)
    }

    pub fn put_bid_tx(&mut self, lot_id: &LotId, tx_hash: Hash) {
        self.bid_history_mut(lot_id).push(tx_hash)
    }

    pub fn add_contract(&mut self, cid: &ContractId, contract: Contract) {
        self.member_contracts_mut(&contract.buyer().id())
            .put(cid, ());
        self.member_contracts_mut(&contract.seller().id())
            .put(cid, ());
        self.update_contract(cid, contract);
    }

    pub fn update_contract(&mut self, cid: &ContractId, contract: Contract) {
        self.contracts_mut().put(cid, contract);
    }

    pub fn remove_contract(&mut self, cid: &ContractId) {
        let contract = self.contracts_mut().get(cid);
        if let Some(contract) = contract {
            self.member_contracts_mut(&contract.buyer().id())
                .remove(cid);
            self.member_contracts_mut(&contract.seller().id())
                .remove(cid);
            self.contracts_mut().remove(cid);
        }
    }

    // pub fn add_contract_tax(&mut self, cid: &ContractId, tax: Tax) {
    //     self.contract_payment_mut()
    //         .insert(tax.payment_number().to_owned());
    //     self.contract_tax_mut().put(cid, tax);
    // }

    pub fn set_check(&mut self, id: &Hash, check: Check) {
        self.checks_mut(id).put(&check.key(), check.result())
    }

    pub fn apply_checks(&mut self, id: &Hash, checks: Vec<Check>) {
        let mut stored_checks = self.checks_mut(id);
        for check in checks {
            stored_checks.put(&check.key(), check.result())
        }
    }

    pub fn clear_checks(&mut self, id: &Hash) {
        self.checks_mut(id).clear()
    }

    // pub fn attach_file(&mut self, member_id: &MemberId, tx_hash: &Hash, doc_hash: Hash) {
    //     self.attachments_mut(member_id).put(tx_hash, doc_hash);
    // }

    pub fn attach_contract_file(
        &mut self,
        contract_id: &ContractId,
        attachment_tx_hash: &Hash,
        attachment_metadata: AttachmentMetadata,
    ) {
        self.contract_files_mut(contract_id)
            .put(attachment_tx_hash, attachment_metadata);
    }

    pub fn attach_contract_notification(
        &mut self,
        contract_id: &ContractId,
        attachment_tx_hash: &Hash,
        attachment_metadata: AttachmentMetadata,
    ) {
        self.contract_notifications_mut(contract_id)
            .put(attachment_tx_hash, attachment_metadata);
    }

    pub fn attach_contract_deed(
        &mut self,
        contract_id: &ContractId,
        tx_hash: &DocumentId,
        file_metadata: AttachmentMetadata,
    ) {
        self.contract_deed_mut().put(
            &contract_id,
            AttachmentMetadataWithHash::new(file_metadata, tx_hash),
        );
    }

    pub fn attach_contract_application(
        &mut self,
        contract_id: &ContractId,
        tx_hash: &DocumentId,
        file_metadata: AttachmentMetadata,
    ) {
        self.contract_application_mut().put(
            &contract_id,
            AttachmentMetadataWithHash::new(file_metadata, tx_hash),
        );
    }

    // pub fn remove_file(&mut self, member_id: &MemberId, document: &DocumentId) {
    //     self.attachments_mut(member_id).remove(document);
    // }

    pub fn remove_contract_file(&mut self, contract_id: &ContractId, document: &DocumentId) {
        self.contract_files_mut(contract_id).remove(document);
    }

    pub fn clear_contract_files(&mut self, contract_id: &ContractId) {
        self.contract_files_mut(contract_id).clear();
        self.contract_deed_mut().remove(contract_id);
        self.contract_application_mut().remove(contract_id);
        self.contract_notifications_mut(contract_id).clear();
    }

    pub fn deprecated_add_sign_contract_tx(&mut self, document_id: &DocumentId, tx_hash: Hash) {
        self.deprecated_sign_contract_tx_mut()
            .put(document_id, tx_hash)
    }

    pub fn add_sign_contract_tx(
        &mut self,
        document_id: &DocumentId,
        member_id: MemberIdentity,
        seller_buyer: BuyerSeller,
        tx_hash: &Hash,
    ) {
        let mut guard = self.sign_contract_tx_mut();
        let contract_to_sign = guard
            .get(document_id)
            .unwrap_or(ContractSign::new(None, None))
            .add_sign(seller_buyer, member_id, tx_hash);
        guard.put(document_id, contract_to_sign)
    }

    pub fn add_participant(&mut self, member_id: &MemberId, node_name: String) {
        self.participants_mut(member_id).push(node_name)
    }

    fn contract_calculations_mut(
        &mut self,
    ) -> MapIndex<&mut Fork, ContractId, PaymentDetailsWrapper> {
        MapIndex::new(CONTRACT_CALCULATIONS_INDEX, &mut self.view)
    }

    pub fn add_contract_calculations(
        &mut self,
        contract_tx_hash: &ContractId,
        calculations: Vec<Calculation>,
    ) {
        self.contract_calculations_mut().put(
            contract_tx_hash,
            calculations
                .into_iter()
                .map(|v| v.into())
                .collect::<Vec<PaymentDetail>>()
                .into(),
        )
    }

    pub fn add_contract_payment_details(
        &mut self,
        contract_tx_hash: &ContractId,
        payment_details: Vec<PaymentDetail>,
    ) {
        self.contract_calculations_mut()
            .put(contract_tx_hash, payment_details.into())
    }

    pub fn change_contract_payment_status(
        &mut self,
        contract_tx_hash: &ContractId,
        payment_id: &str,
        status: PaymentStatus,
    ) -> Result<()> {
        let mut index = self.contract_calculations_mut();
        let calculations = index
            .get(contract_tx_hash)
            .ok_or_else(|| Error::no_calculation_for_contract("none", contract_tx_hash))?;

        let new_value = calculations
            .set_payment_status(payment_id, status)
            .ok_or_else(|| Error::no_calculation_for_contract(payment_id, contract_tx_hash))?;
        index.put(contract_tx_hash, new_value);
        Ok(())
    }

    fn lot_calculations_mut(&mut self) -> MapIndex<&mut Fork, ContractId, PaymentDetailsWrapper> {
        MapIndex::new(LOT_CALCULATIONS_INDEX, &mut self.view)
    }

    pub fn add_lot_calculations(&mut self, lot_tx_hash: &LotId, calculations: Vec<Calculation>) {
        self.lot_calculations_mut().put(
            lot_tx_hash,
            calculations
                .into_iter()
                .map(|v| v.into())
                .collect::<Vec<PaymentDetail>>()
                .into(),
        )
    }

    fn remove_lot_calculations(&mut self, lot_tx_hash: &LotId) {
        self.lot_calculations_mut().remove(lot_tx_hash)
    }
}
