use blockp_core::crypto::Hash;

#[repr(u8)]
pub enum RequestKind {
    AddObject,
    AddObjectGroup,
}

encoding_struct! {
    struct Request {
        kind: u8,
        tx_hash: &Hash
    }
}

impl Request {
    pub fn add_object(tx_hash: &Hash) -> Request {
        Request::new(RequestKind::AddObject as u8, tx_hash)
    }

    pub fn add_object_group(tx_hash: &Hash) -> Request {
        Request::new(RequestKind::AddObjectGroup as u8, tx_hash)
    }
}
