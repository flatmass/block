use super::conditions::Conditions;
use super::member::MemberIdentity;
use crate::error::Error;
use blockp_core::crypto::Hash;
use chrono::{DateTime, Utc};
use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};

pub type ContractId = Hash;

encoding_struct! {
    struct Contract {
        buyer: MemberIdentity,
        seller: MemberIdentity,
        price: u64,
        conditions: Conditions,
        state: u16,
    }
}

impl Contract {
    pub fn buy(
        buyer: MemberIdentity,
        seller: MemberIdentity,
        price: u64,
        conditions: Conditions,
    ) -> Self {
        Self::new(buyer, seller, price, conditions, State::NEW.bits)
    }

    pub fn is_member(&self, member: &MemberIdentity) -> bool {
        self.buyer() == *member || self.seller() == *member
    }

    pub fn check_modifiable(&self) -> Result<(), Error> {
        let status = ContractStatus::try_from(self.state())?;
        match status {
            // Required to allow initial document attachment
            ContractStatus::New => Ok(()),
            ContractStatus::Draft(_) => Ok(()),
            _ => Error::bad_contract_state(status, "Update").ok(),
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

    pub fn apply(self, action: Action) -> Result<Self, Error> {
        use Action::*;
        use ContractStatus as Status;
        let status = ContractStatus::try_from(self.state())?;
        match (status, action.clone()) {
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
            (Status::Confirmed(c), Action::Sign(u)) if self.buyer() == u => {
                Ok(self.set(c.sign_buyer().into()))
            }
            (Status::Confirmed(c), Action::Sign(u)) if self.seller() == u => {
                Ok(self.set(c.sign_seller().into()))
            }
            (Status::Signed, Register) => Ok(self.set(Status::Registering)),
            (Status::Registering, AwaitUserAction) => Ok(self.set(Status::AwaitingUserAction)),
            (Status::Registering, Reject) => Ok(self.set(Status::Rejected)),
            (Status::Registering, Approve) => Ok(self.set(Status::Approved)),
            (Status::AwaitingUserAction, Reject) => Ok(self.set(Status::Rejected)),
            (Status::AwaitingUserAction, Approve) => Ok(self.set(Status::Approved)),
            _ => Error::bad_contract_state(status, &format!("{:?}", action)).ok(),
        }
    }

    fn modify(self, price: u64, conditions: Conditions, state: u16) -> Self {
        Self::new(self.buyer(), self.seller(), price, conditions, state)
    }

    fn set(self, status: ContractStatus) -> Self {
        Self::new(
            self.buyer(),
            self.seller(),
            self.price(),
            self.conditions(),
            State::from(status).bits,
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
pub enum ContractStatus {
    New,
    Draft(Draft),
    Confirmed(Confirmed),
    Signed,
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
        } else {
            Err(Error::internal_bad_struct("ContractStatus"))?
        };
        Ok(status)
    }
}

impl From<ContractStatus> for State {
    fn from(status: ContractStatus) -> State {
        match status {
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
                Self::New => "new",
                Self::Draft(_) => "draft",
                Self::Confirmed(_) => "confirmed",
                Self::Signed => "signed",
                Self::Refused => "refused",
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
    MakeDraft,
    Confirm(MemberIdentity),
    Sign(MemberIdentity),
    Update { price: u64, conditions: Conditions },
    Refuse,
    Approve,
    Reject,
    Register,
    AwaitUserAction,
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
    }
}

encoding_struct! {
    struct Tax {
        requestor: MemberIdentity,
        payment_number: &str,
        payment_date: DateTime<Utc>,
        amount: u64,
    }
}
