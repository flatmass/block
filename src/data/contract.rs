use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};

use blockp_core::crypto::Hash;

use crate::error::Error;

use super::conditions::Conditions;
use super::member::MemberIdentity;

pub type ContractId = Hash;

encoding_struct! {
    struct Contract {
        buyer: MemberIdentity,
        seller: MemberIdentity,
        price: u64,
        conditions: Conditions,
        state: u16,
        /// something has been changed with objects while contract was opened
        undefined: bool
    }
}

encoding_struct! {
    struct ContractSign {
        buyer_sign_tx_hash: Option<MemberIdentityDocSign>,
        seller_sign_tx_hash: Option<MemberIdentityDocSign>,
    }
}

impl ContractSign {
    pub fn add_sign(
        self,
        seller_buyer: BuyerSeller,
        member_id: MemberIdentity,
        tx_hash: &Hash,
    ) -> ContractSign {
        match seller_buyer {
            BuyerSeller::Buyer => ContractSign::new(
                Some(MemberIdentityDocSign::new(member_id, tx_hash)),
                self.seller_sign_tx_hash(),
            ),
            BuyerSeller::Seller => ContractSign::new(
                self.buyer_sign_tx_hash(),
                Some(MemberIdentityDocSign::new(member_id, tx_hash)),
            ),
        }
    }
}

pub enum BuyerSeller {
    Buyer,
    Seller,
}

encoding_struct! {
    struct MemberIdentityDocSign {
        signer: MemberIdentity,
        sign_tx_hash: &Hash,
    }
}

impl Contract {
    pub fn buy(
        buyer: MemberIdentity,
        seller: MemberIdentity,
        price: u64,
        conditions: Conditions,
    ) -> Self {
        Self::new(
            buyer,
            seller,
            price,
            conditions,
            (State::REQUEST_CONFIRM | State::BUYER_PROCEEDED).bits,
            false,
        )
    }

    pub fn sell(
        buyer: MemberIdentity,
        seller: MemberIdentity,
        price: u64,
        conditions: Conditions,
    ) -> Self {
        Self::new(
            buyer,
            seller,
            price,
            conditions,
            (State::REQUEST_CONFIRM | State::SELLER_PROCEEDED).bits,
            false,
        )
    }

    pub fn is_member(&self, member: &MemberIdentity) -> bool {
        self.buyer() == *member || self.seller() == *member
    }

    pub fn is_buyer(&self, member: &MemberIdentity) -> bool {
        self.buyer() == *member
    }

    pub fn is_seller(&self, member: &MemberIdentity) -> bool {
        self.seller() == *member
    }

    pub fn is_draft(&self) -> Result<bool, Error> {
        let status = ContractStatus::try_from(self.state())?;
        match status {
            ContractStatus::Draft(_) => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn is_finished(&self) -> Result<bool, Error> {
        let status = ContractStatus::try_from(self.state())?;
        match status {
            ContractStatus::Refused | ContractStatus::Approved | ContractStatus::Rejected => {
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub fn check_modifiable(&self) -> Result<bool, Error> {
        let status = ContractStatus::try_from(self.state())?;
        match status {
            // Required to allow initial document attachment
            ContractStatus::New => Ok(true),
            ContractStatus::Draft(_) => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn check_can_add_tax(&self) -> Result<(), Error> {
        let status = ContractStatus::try_from(self.state())?;
        match status {
            ContractStatus::Draft(_) => Ok(()),
            ContractStatus::Confirmed(_) => Ok(()),
            _ => Error::bad_contract_state(status, "AddTax").ok(),
        }
    }

    pub fn is_signed(&self) -> Result<bool, Error> {
        let status = ContractStatus::try_from(self.state())?;
        match status {
            ContractStatus::Signed => Ok(true),
            _ => Ok(false),
        }
    }

    pub fn is_undefined(&self) -> bool {
        self.undefined()
    }

    pub fn set_undefined(self, undefined: bool) -> Self {
        Self::new(
            self.buyer(),
            self.seller(),
            self.price(),
            self.conditions(),
            self.state(),
            undefined,
        )
    }

    pub fn apply(self, action: Action) -> Result<Self, Error> {
        use Action::*;
        use ContractStatus as Status;
        let status = ContractStatus::try_from(self.state())?;
        match (status, action.clone()) {
            (Status::RequestConfirm(c), Confirm(u)) if self.buyer() == u => {
                Ok(self.set(c.confirm_buyer().into()))
            }
            (Status::RequestConfirm(c), Unconfirm(u)) if self.buyer() == u => {
                Ok(self.set(c.unconfirm_buyer().into()))
            }
            (Status::RequestConfirm(c), Confirm(u)) if self.seller() == u => {
                Ok(self.set(c.confirm_seller().into()))
            }
            (Status::RequestConfirm(c), Unconfirm(u)) if self.seller() == u => {
                Ok(self.set(c.unconfirm_seller().into()))
            }
            (
                Status::RequestConfirm(RequestConfirm {
                    buyer: true,
                    seller: true,
                }),
                New,
            ) => Ok(self.set(Status::New)),
            (Status::New, MakeDraft) => Ok(self.set(Status::Draft(Draft::new()))),
            (Status::New, Reject) => Ok(self.set(Status::Rejected)),
            (Status::Draft(_), Update { price, conditions }) => {
                Ok(self.modify(price, conditions, State::NEW.bits))
            }
            (Status::Draft(_), Refuse) => Ok(self.set(Status::Refused)),
            (Status::Draft(d), Confirm(u)) if self.buyer() == u => {
                Ok(self.set(d.confirm_buyer().into()))
            }
            (Status::Draft(d), Confirm(u)) if self.seller() == u => {
                Ok(self.set(d.confirm_seller().into()))
            }
            (Status::Draft(_), MakeDraft) => Ok(self.set(Status::Draft(Draft::new()))),
            (Status::Confirmed(_), Refuse) => Ok(self.set(Status::Refused)),
            (Status::Confirmed(_), MakeDraft) => Ok(self.set(Status::Draft(Draft::new()))),
            (Status::Confirmed(_), Update { price, conditions }) => {
                Ok(self.modify(price, conditions, State::NEW.bits))
            }
            (Status::Confirmed(c), Action::Sign(u)) if self.buyer() == u => {
                Ok(self.set(c.sign_buyer().into()))
            }
            (Status::Confirmed(c), Action::Sign(u)) if self.seller() == u => {
                Ok(self.set(c.sign_seller().into()))
            }
            (Status::Signed, Refuse) => Ok(self.set(Status::Refused)),
            (Status::Signed, ReadyForRegistering) => Ok(self.set(Status::ReadyForRegistering)),
            (Status::ReadyForRegistering, Register) => Ok(self.set(Status::Registering)),
            (Status::Registering, AwaitUserAction) => Ok(self.set(Status::AwaitingUserAction)),
            (Status::Registering, Reject) => Ok(self.set(Status::Rejected)),
            (Status::Registering, Approve) => Ok(self.set(Status::Approved)),
            (Status::AwaitingUserAction, Reject) => Ok(self.set(Status::Rejected)),
            (Status::AwaitingUserAction, Approve) => Ok(self.set(Status::Approved)),
            _ => Error::bad_contract_state(status, &format!("{:?}", action)).ok(),
        }
    }

    fn modify(self, price: u64, conditions: Conditions, state: u16) -> Self {
        Self::new(
            self.buyer(),
            self.seller(),
            price,
            conditions,
            state,
            self.undefined(),
        )
    }

    fn set(self, status: ContractStatus) -> Self {
        Self::new(
            self.buyer(),
            self.seller(),
            self.price(),
            self.conditions(),
            State::from(status).bits,
            self.undefined(),
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Draft {
    buyer: bool,
    seller: bool,
}

impl Draft {
    pub fn new() -> Self {
        Draft {
            buyer: false,
            seller: false,
        }
    }

    fn confirm_buyer(self) -> ContractStatus {
        if self.seller {
            ContractStatus::Confirmed(Confirmed::new())
        } else {
            ContractStatus::Draft(Draft {
                buyer: true,
                ..self
            })
        }
    }

    fn confirm_seller(self) -> ContractStatus {
        if self.buyer {
            ContractStatus::Confirmed(Confirmed::new())
        } else {
            ContractStatus::Draft(Draft {
                seller: true,
                ..self
            })
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Confirmed {
    buyer: bool,
    seller: bool,
}

impl Confirmed {
    fn new() -> Self {
        Confirmed {
            buyer: false,
            seller: false,
        }
    }

    fn sign_buyer(mut self) -> Self {
        self.buyer = true;
        self
    }

    fn sign_seller(mut self) -> Self {
        self.seller = true;
        self
    }
}

impl From<Confirmed> for ContractStatus {
    fn from(confirmed: Confirmed) -> Self {
        if confirmed.buyer && confirmed.seller {
            ContractStatus::Signed
        } else {
            ContractStatus::Confirmed(confirmed)
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct RequestConfirm {
    pub buyer: bool,
    pub seller: bool,
}

impl RequestConfirm {
    pub fn is_buyer(&self) -> &bool {
        &self.buyer
    }

    pub fn is_seller(&self) -> &bool {
        &self.seller
    }

    fn confirm_buyer(mut self) -> Self {
        self.buyer = true;
        self
    }

    fn confirm_seller(mut self) -> Self {
        self.seller = true;
        self
    }

    fn unconfirm_buyer(mut self) -> Self {
        self.buyer = false;
        self
    }

    fn unconfirm_seller(mut self) -> Self {
        self.seller = false;
        self
    }
}

impl From<RequestConfirm> for ContractStatus {
    fn from(request_confirm: RequestConfirm) -> Self {
        ContractStatus::RequestConfirm(request_confirm)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ContractStatus {
    RequestConfirm(RequestConfirm),
    New,
    Draft(Draft),
    Confirmed(Confirmed),
    Signed,
    ReadyForRegistering,
    Registering,
    AwaitingUserAction,
    Refused,
    Approved,
    Rejected,
}

impl TryFrom<u16> for ContractStatus {
    type Error = Error;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        let state = State::from_bits(value)
            .ok_or_else(|| Error::bad_state("stored contract state is invalid"))?;
        let status = if state.is_empty() {
            ContractStatus::New
        } else if state == State::REJECTED {
            ContractStatus::Rejected
        } else if state == State::APPROVED {
            ContractStatus::Approved
        } else if state == State::REFUSED {
            ContractStatus::Refused
        } else if state == State::AWAITING_USER_ACTION {
            ContractStatus::AwaitingUserAction
        } else if state == State::REGISTERING {
            ContractStatus::Registering
        } else if state == State::SIGNED | State::PAID {
            ContractStatus::ReadyForRegistering
        } else if state == State::SIGNED {
            ContractStatus::Signed
        } else if state.contains(State::CONFIRMED) {
            let c = Confirmed {
                buyer: state.contains(State::BUYER_PROCEEDED),
                seller: state.contains(State::SELLER_PROCEEDED),
            };
            ContractStatus::Confirmed(c)
        } else if state.contains(State::DRAFT) {
            let d = Draft {
                buyer: state.contains(State::BUYER_PROCEEDED),
                seller: state.contains(State::SELLER_PROCEEDED),
            };
            ContractStatus::Draft(d)
        } else if state.contains(State::REQUEST_CONFIRM) {
            let c = RequestConfirm {
                buyer: state.contains(State::BUYER_PROCEEDED),
                seller: state.contains(State::SELLER_PROCEEDED),
            };
            ContractStatus::RequestConfirm(c)
        } else {
            Err(Error::internal_bad_struct("ContractStatus"))?
        };
        Ok(status)
    }
}

impl From<ContractStatus> for State {
    fn from(status: ContractStatus) -> State {
        match status {
            ContractStatus::RequestConfirm(RequestConfirm { buyer, seller }) => {
                let mut state = State::REQUEST_CONFIRM;
                if buyer {
                    state.insert(State::BUYER_PROCEEDED)
                }
                if seller {
                    state.insert(State::SELLER_PROCEEDED)
                }
                state
            }
            ContractStatus::New => State::NEW,
            ContractStatus::Draft(Draft { buyer, seller }) => {
                let mut state = State::DRAFT;
                if buyer {
                    state.insert(State::BUYER_PROCEEDED)
                }
                if seller {
                    state.insert(State::SELLER_PROCEEDED)
                }
                state
            }
            ContractStatus::Confirmed(Confirmed { buyer, seller }) => {
                let mut state = State::CONFIRMED;
                if buyer {
                    state.insert(State::BUYER_PROCEEDED)
                }
                if seller {
                    state.insert(State::SELLER_PROCEEDED)
                }
                state
            }
            ContractStatus::Refused => State::REFUSED,
            ContractStatus::Signed => State::SIGNED,
            ContractStatus::ReadyForRegistering => State::SIGNED | State::PAID,
            ContractStatus::Registering => State::REGISTERING,
            ContractStatus::AwaitingUserAction => State::AWAITING_USER_ACTION,
            ContractStatus::Rejected => State::REJECTED,
            ContractStatus::Approved => State::APPROVED,
        }
    }
}

impl Display for ContractStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::RequestConfirm(_) => "request_confirm",
                Self::New => "new",
                Self::Draft(_) => "draft",
                Self::Confirmed(_) => "confirmed",
                Self::Signed => "signed",
                Self::Refused => "refused",
                Self::ReadyForRegistering => "ready_for_registering",
                Self::Registering => "registering",
                Self::AwaitingUserAction => "awaiting_user_action",
                Self::Approved => "approved",
                Self::Rejected => "rejected",
            }
        )
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    New,
    MakeDraft,
    Confirm(MemberIdentity),
    Unconfirm(MemberIdentity),
    Sign(MemberIdentity),
    Update { price: u64, conditions: Conditions },
    Refuse,
    Approve,
    Reject,
    Register,
    AwaitUserAction,
    ReadyForRegistering,
}

bitflags! {
    pub struct State : u16 {
        const NEW = 0;
        const DRAFT = 1;
        const CONFIRMED = 2;
        const BUYER_PROCEEDED = 4;
        const SELLER_PROCEEDED = 8;
        const SIGNED = 16;
        const REGISTERING = 32;
        const REFUSED = 64;
        const APPROVED = 128;
        const REJECTED = 256;
        const AWAITING_USER_ACTION = 512;
        const PAID = 1024;
        const REQUEST_CONFIRM = 2048;
    }
}

encoding_struct! {
    struct CorrespondenceContacts {
        contract_correspondence: Option<String>,
        objects_correspondence: Option<String>,
    }
}
