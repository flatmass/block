#![allow(dead_code)]

use std::collections::HashMap;

use blockp_core::blockchain::Schema as CoreSchema;
use blockp_core::crypto::{Hash, PublicKey};
use blockp_core::messages::RawMessage;
use blockp_core::storage::{Fork, ListIndex, MapIndex, ProofListIndex, Snapshot, ValueSetIndex};

use crate::data::attachment::{DocumentId, Sign};
use crate::data::conditions::{Check, CheckResult, Conditions};
use crate::data::contract::{Contract, ContractId, Tax};
use crate::data::lot::{Bid, Lot, LotId, LotState, LotStatus};
use crate::data::member::MemberId;
use crate::data::object::{Change, ObjectId, ObjectIdentity};
use crate::data::ownership::{OwnershipUnstructured, Rights};
use crate::data::request::Request;
use crate::error::{Error, Result};

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

    pub fn object_locks(&self, object_id: &ObjectId) -> MapIndex<&T, MemberId, bool> {
        MapIndex::new_in_family("fips.object_locks", object_id, &self.view)
    }

    pub fn rightholders(&self, object_id: &ObjectId) -> MapIndex<&T, MemberId, Rights> {
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

    /*pub fn is_published(&self, object_id: &ObjectId) -> bool {
        self.object_publications(object_id).iter().next().is_some()
    }*/

    pub fn lots(&self) -> MapIndex<&T, LotId, Lot> {
        MapIndex::new("fips.lots", &self.view)
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

    pub fn contract_tax(&self) -> MapIndex<&T, ContractId, Tax> {
        MapIndex::new("fips.contract_tax", &self.view)
    }

    pub fn contract_payment(&self) -> ValueSetIndex<&T, String> {
        ValueSetIndex::new("fips.tax_payments", &self.view)
    }

    pub fn checks(&self, contract_id: &ContractId) -> MapIndex<&T, u16, CheckResult> {
        MapIndex::new_in_family("fips.checks", contract_id, &self.view)
    }

    pub fn member_contracts(&self, member_id: &MemberId) -> MapIndex<&T, ContractId, ()> {
        MapIndex::new_in_family("fips.member_contracts", member_id, &self.view)
    }

    pub fn attachments(&self, member_id: &MemberId) -> MapIndex<&T, DocumentId, Hash> {
        MapIndex::new_in_family("fips.attachments", member_id, &self.view)
    }

    pub fn contract_files(&self, contract_id: &ContractId) -> MapIndex<&T, ContractId, Hash> {
        MapIndex::new_in_family("fips.contract_files", contract_id, &self.view)
    }

    pub fn contract_deed(&self, contract_id: &ContractId) -> Option<Hash> {
        let storage: MapIndex<&T, ContractId, Hash> =
            MapIndex::new("fips.contract_files.deed", &self.view);
        storage.get(contract_id)
    }

    pub fn contract_application(&self, contract_id: &ContractId) -> Option<Hash> {
        let storage: MapIndex<&T, ContractId, Hash> =
            MapIndex::new("fips.contract_files.application", &self.view);
        storage.get(contract_id)
    }

    pub fn attachment_signs(&self, doc_tx_hash: &Hash) -> MapIndex<&T, MemberId, Sign> {
        MapIndex::new_in_family("fips.attachment_signs", doc_tx_hash, &self.view)
    }

    pub fn requests(&self, member_id: &MemberId) -> ListIndex<&T, Request> {
        ListIndex::new_in_family("fips.requests", member_id, &self.view)
    }

    pub fn participants(&self, member_id: &MemberId) -> ListIndex<&T, String> {
        ListIndex::new_in_family("fips.participants", member_id, &self.view)
    }

    // core as a dependency
    pub fn core_transactions(&self) -> MapIndex<&T, Hash, RawMessage> {
        MapIndex::new("core.transactions", &self.view)
    }

    // TODO time service as a dependency
    /*pub fn time(&self) -> Entry<&T, DateTime<Utc>> {
        Entry::new("exonum_time.time", &self.view)
    }*/

    pub fn is_owner(&self, member_id: &MemberId, obj_id: &ObjectId) -> bool {
        self.rightholders(obj_id)
            .get(member_id)
            .map(|rights| rights.is_owner())
            .unwrap_or_default()
    }

    pub fn find_owner(&self, obj_id: &ObjectId) -> Option<MemberId> {
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

    pub fn rights(&self, member_id: &MemberId, obj_id: &ObjectId) -> Option<Rights> {
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
}

impl<'a> Schema<&'a mut Fork> {
    fn objects_mut(&mut self) -> MapIndex<&mut Fork, ObjectId, String> {
        MapIndex::new("fips.objects", &mut self.view)
    }

    fn object_locks_mut(&mut self, object_id: &ObjectId) -> MapIndex<&mut Fork, MemberId, i32> {
        MapIndex::new_in_family("fips.object_locks", object_id, &mut self.view)
    }

    fn rightholders_mut(&mut self, object_id: &ObjectId) -> MapIndex<&mut Fork, MemberId, Rights> {
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

    fn lots_mut(&mut self) -> MapIndex<&mut Fork, LotId, Lot> {
        MapIndex::new("fips.lots", &mut self.view)
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

    fn contract_tax_mut(&mut self) -> MapIndex<&mut Fork, ContractId, Tax> {
        MapIndex::new("fips.contract_tax", &mut self.view)
    }

    fn contract_payment_mut(&mut self) -> ValueSetIndex<&mut Fork, String> {
        ValueSetIndex::new("fips.tax_payments", &mut self.view)
    }

    fn checks_mut(&mut self, contract_id: &ContractId) -> MapIndex<&mut Fork, u16, CheckResult> {
        MapIndex::new_in_family("fips.checks", contract_id, &mut self.view)
    }

    fn member_contracts_mut(
        &mut self,
        member_id: &MemberId,
    ) -> MapIndex<&mut Fork, ContractId, ()> {
        MapIndex::new_in_family("fips.member_contracts", member_id, &mut self.view)
    }

    fn attachments_mut(&mut self, member_id: &MemberId) -> MapIndex<&mut Fork, DocumentId, Hash> {
        MapIndex::new_in_family("fips.attachments", member_id, &mut self.view)
    }

    fn contract_files_mut(
        &mut self,
        contract_id: &ContractId,
    ) -> MapIndex<&mut Fork, DocumentId, ()> {
        MapIndex::new_in_family("fips.contract_files", contract_id, &mut self.view)
    }

    fn contract_deed_mut(&mut self) -> MapIndex<&mut Fork, ContractId, DocumentId> {
        MapIndex::new("fips.contract_files.deed", &mut self.view)
    }

    fn contract_application_mut(&mut self) -> MapIndex<&mut Fork, ContractId, DocumentId> {
        MapIndex::new("fips.contract_files.application", &mut self.view)
    }

    fn attachment_signs_mut(
        &mut self,
        doc_tx_hash: &DocumentId,
    ) -> MapIndex<&mut Fork, MemberId, Sign> {
        MapIndex::new_in_family("fips.attachment_signs", doc_tx_hash, self.view)
    }

    fn requests_mut(&mut self, member_id: &MemberId) -> ListIndex<&mut Fork, Request> {
        ListIndex::new_in_family("fips.requests", member_id, &mut self.view)
    }

    fn participants_mut(&mut self, member_id: &MemberId) -> ListIndex<&mut Fork, String> {
        ListIndex::new_in_family("fips.participants", member_id, &mut self.view)
    }

    pub fn contract_reference_number_mut(&mut self) -> MapIndex<&mut Fork, ContractId, String> {
        MapIndex::new("fips.contracts.reference_number", &mut self.view)
    }

    pub fn update_object_history(&mut self, object_id: &ObjectId, change: Change) -> Hash {
        let mut history = self.object_history_mut(object_id);
        history.push(change);
        history.merkle_root()
    }

    fn invalidate_published_lots(&mut self, object_id: &ObjectId) {
        let states = self
            .object_publications(object_id)
            .iter()
            .filter_map(|(lot_id, _)| {
                let state = self.lot_states().get(&lot_id).unwrap();
                // TODO need to check conditions
                if !state.is_executed() {
                    Some((lot_id, state.set_status(LotStatus::Undefined)))
                } else if !state.is_closed() {
                    Some((lot_id, state.set_status(LotStatus::Closed)))
                } else {
                    None
                }
            })
            .collect::<Vec<(LotId, LotState)>>();
        for (lot_id, state) in states {
            self.set_lot_state(&lot_id, state);
        }
    }

    pub fn update_rights(&mut self, object: &ObjectIdentity, rights: HashMap<MemberId, Rights>) {
        let object_id = &object.id();
        let to_remove = self
            .rightholders(object_id)
            .keys()
            .filter(|uid| !rights.contains_key(uid))
            .collect::<Vec<MemberId>>();
        for uid in rights.keys() {
            info!("added ownership {:?}", uid);
            self.ownership_mut(&uid).insert(object.clone());
        }
        for uid in to_remove.iter() {
            info!("removed ownership {:?}", uid);
            self.ownership_mut(&uid).remove(&object);
        }
        self.invalidate_published_lots(object_id);
        let mut rightholders = self.rightholders_mut(object_id);
        for uid in to_remove.iter() {
            info!("removed {:?}", uid);
            rightholders.remove(&uid);
        }
        for (uid, rights) in rights.into_iter() {
            info!("added {:?} : {:?}", uid, rights);
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
        self.invalidate_published_lots(object_id);
    }

    pub fn update_object_data(&mut self, obj_id: &ObjectId, data: &str, tx_hash: &Hash) {
        let change = Change::new(tx_hash);
        self.objects_mut().put(obj_id, data.to_string());
        self.update_object_history(obj_id, change);
    }

    pub fn set_published(&mut self, object_id: &ObjectId, lot_id: &LotId) {
        self.object_publications_mut(object_id).put(lot_id, ())
    }

    pub fn set_unpublished(&mut self, object_id: &ObjectId, lot_id: &LotId) {
        self.object_publications_mut(object_id).remove(lot_id)
    }

    pub fn add_lot(&mut self, lot_id: &LotId, lot: Lot, conditions: Conditions) {
        self.lots_mut().put(lot_id, lot);
        self.lot_conditions_mut().put(lot_id, conditions);
    }

    pub fn remove_lot(&mut self, lot_id: &LotId) {
        self.lots_mut().remove(lot_id);
        self.lot_conditions_mut().remove(lot_id);
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
            &mut self.contracts_mut().remove(cid);
        }
    }

    pub fn add_contract_tax(&mut self, cid: &ContractId, tax: Tax) {
        self.contract_payment_mut()
            .insert(tax.payment_number().to_owned());
        self.contract_tax_mut().put(cid, tax);
    }

    pub fn set_check(&mut self, cid: &ContractId, check: Check) {
        self.checks_mut(cid).put(&check.key(), check.result())
    }

    pub fn apply_checks(&mut self, cid: &ContractId, checks: Vec<Check>) {
        let mut stored_checks = self.checks_mut(cid);
        for check in checks {
            stored_checks.put(&check.key(), check.result())
        }
    }

    pub fn clear_checks(&mut self, cid: &ContractId) {
        self.checks_mut(cid).clear()
    }

    pub fn attach_file(&mut self, member_id: &MemberId, tx_hash: &Hash, doc_hash: Hash) {
        self.attachments_mut(member_id).put(tx_hash, doc_hash);
    }

    pub fn attach_contract_file(&mut self, contract_id: &ContractId, tx_hash: &Hash) {
        self.contract_files_mut(contract_id).put(tx_hash, ());
    }

    pub fn attach_contract_deed(&mut self, contract_id: &ContractId, tx_hash: Hash) {
        self.contract_deed_mut().put(&contract_id, tx_hash);
    }

    pub fn attach_contract_application(&mut self, contract_id: &ContractId, tx_hash: Hash) {
        self.contract_application_mut().put(&contract_id, tx_hash);
    }

    pub fn remove_file(&mut self, member_id: &MemberId, document: &DocumentId) {
        self.attachments_mut(member_id).remove(document);
    }

    pub fn remove_contract_file(&mut self, contract_id: &ContractId, document: &DocumentId) {
        self.contract_files_mut(contract_id).remove(document);
    }

    pub fn clear_contract_files(&mut self, contract_id: &ContractId) {
        self.contract_files_mut(contract_id).clear();
        self.contract_deed_mut().remove(contract_id);
        self.contract_application_mut().remove(contract_id);
    }

    pub fn add_attachment_sign(&mut self, doc_tx_hash: &Hash, member_id: &MemberId, sign: Sign) {
        self.attachment_signs_mut(doc_tx_hash).put(member_id, sign)
    }

    pub fn remove_attachment_sign(&mut self, member_id: &MemberId, doc_tx_hash: &Hash) {
        self.attachment_signs_mut(doc_tx_hash).remove(member_id);
    }

    pub fn put_request(&mut self, member_id: &MemberId, request: Request) {
        self.requests_mut(member_id).push(request)
    }

    pub fn add_participant(&mut self, member_id: &MemberId, node_name: String) {
        self.participants_mut(member_id).push(node_name)
    }

    pub fn lock_object(
        &mut self,
        member_id: &MemberId,
        object_id: &ObjectId,
        exclusive: bool,
    ) -> bool {
        let mut locks = self.object_locks_mut(object_id);
        let value = locks.get(member_id).unwrap_or_default();
        if value != 0 && exclusive || value < 0 {
            return false;
        }
        let value = if exclusive { -1 } else { value + 1 };
        locks.put(member_id, value);
        true
    }

    pub fn unlock_object(&mut self, member_id: &MemberId, object_id: &ObjectId) -> bool {
        let mut locks = self.object_locks_mut(object_id);
        let value = locks.get(member_id).unwrap_or_default();
        // no locks
        if value != 0 {
            let value = if value < 0 { 0 } else { value - 1 };
            locks.put(member_id, value);
        }
        true // TODO
    }

    pub fn set_contract_reference_number(
        &mut self,
        contract_id: &ContractId,
        reference_number: String,
    ) {
        let mut storage: MapIndex<&mut Fork, ContractId, String> =
            MapIndex::new("fips.contracts.reference_number", &mut self.view);
        storage.put(contract_id, reference_number)
    }
}
