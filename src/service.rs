use blockp_core::{
    api::ServiceApiBuilder,
    blockchain::{Service, Transaction, TransactionSet},
    crypto::Hash,
    encoding::Error as EncodingError,
    helpers::fabric::{self, Context},
    messages::RawTransaction,
    storage::Fork,
    storage::Snapshot,
};

use crate::api::OwnershipApi;
use crate::schema::Schema;
use crate::transactions::OwnershipTransactions;

/// Unique service ID.
pub(crate) const SERVICE_ID: u16 = 130;
/// Name of the service.
const SERVICE_NAME: &str = "fips-ownership";

/// Exonum `Service` implementation.
#[derive(Default, Debug)]
pub struct OwnershipService;

impl Service for OwnershipService {
    fn service_name(&self) -> &str {
        SERVICE_NAME
    }

    fn service_id(&self) -> u16 {
        SERVICE_ID
    }

    fn state_hash(&self, view: &dyn Snapshot) -> Vec<Hash> {
        let schema = Schema::new(view);
        schema.state_hash()
    }

    fn clear(&self, _fork: &mut Fork) {
        unimplemented!()
    }

    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<dyn Transaction>, EncodingError> {
        OwnershipTransactions::tx_from_raw(raw).map(Into::into)
    }

    fn wire_api(&self, builder: &mut ServiceApiBuilder) {
        OwnershipApi::wire(builder);
    }
}

#[derive(Debug)]
pub struct ServiceFactory;

impl fabric::ServiceFactory for ServiceFactory {
    fn service_name(&self) -> &str {
        SERVICE_NAME
    }

    fn make_service(&mut self, _: &Context) -> Box<dyn Service> {
        Box::new(OwnershipService)
    }
}
