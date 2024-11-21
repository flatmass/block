use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error;
use crate::error::Error;

encoding_struct! {
    struct Calculation {
        id: &str,
        data: &str,
        timestamp: DateTime<Utc>,
    }
}

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    NotPaid = 0,
    Paid = 1,
    Cancelled = 2,
}

impl FromStr for PaymentStatus {
    type Err = Error;

    fn from_str(src: &str) -> error::Result<Self> {
        serde_plain::from_str(src).map_err(|_| Error::bad_payment_status(src))
    }
}

impl TryFrom<u8> for PaymentStatus {
    type Error = Error;

    fn try_from(val: u8) -> error::Result<Self> {
        let result = match val {
            0 => PaymentStatus::NotPaid,
            1 => PaymentStatus::Paid,
            2 => PaymentStatus::Cancelled,
            _ => return Err(Error::unexpected_payment_status()),
        };
        Ok(result)
    }
}

encoding_struct! {
    struct PaymentDetail {
        calculation: Calculation,
        payment_detail: &str,
        status: u8
    }
}

impl PaymentDetail {
    pub fn is_paid(&self) -> error::Result<bool> {
        match self.status().try_into()? {
            PaymentStatus::Paid => Ok(true),
            _ => Ok(false),
        }
    }
}

impl From<Calculation> for PaymentDetail {
    fn from(v: Calculation) -> Self {
        PaymentDetail::new(v, "", PaymentStatus::NotPaid as u8)
    }
}

impl From<PaymentDetail> for Calculation {
    fn from(v: PaymentDetail) -> Self {
        Calculation::new(
            v.calculation().id(),
            v.calculation().data(),
            v.calculation().timestamp(),
        )
    }
}

encoding_struct! {
    struct PaymentDetailsWrapper {
        payment_details: Vec<PaymentDetail>,
    }
}

impl PaymentDetailsWrapper {
    pub fn set_payment_status(
        &self,
        payment_id: &str,
        new_status: PaymentStatus,
    ) -> Option<PaymentDetailsWrapper> {
        let mut payment_details = self.payment_details();
        let value = payment_details
            .iter_mut()
            .find(|payment_detail| payment_detail.calculation().id() == payment_id);

        if let Some(payment_detail) = value {
            let new_value = PaymentDetail::new(
                payment_detail.calculation(),
                payment_detail.payment_detail(),
                new_status as u8,
            );
            *payment_detail = new_value;
        } else {
            return None;
        }

        Some(payment_details.into())
    }

    pub fn get_all_paid(self) -> Vec<PaymentDetail> {
        self.payment_details()
            .into_iter()
            .filter(|x| match x.is_paid() {
                Ok(x) => x,
                Err(_) => false,
            })
            .collect()
    }
}

impl From<Vec<PaymentDetail>> for PaymentDetailsWrapper {
    fn from(v: Vec<PaymentDetail>) -> Self {
        PaymentDetailsWrapper::new(v)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    #[test]
    pub fn test_set_payment_status() {
        let payment_details = vec![
            PaymentDetail::new(
                Calculation::new("1", "test1", Utc::now()),
                "test1",
                PaymentStatus::NotPaid as u8,
            ),
            PaymentDetail::new(
                Calculation::new("2", "test2", Utc::now()),
                "test2",
                PaymentStatus::NotPaid as u8,
            ),
            PaymentDetail::new(
                Calculation::new("3", "test3", Utc::now()),
                "test3",
                PaymentStatus::NotPaid as u8,
            ),
        ];
        let start_value = PaymentDetailsWrapper::new(payment_details);
        let payment_id_for_change = "3";
        let new_status = PaymentStatus::Paid;
        let new_value = start_value
            .clone()
            .set_payment_status(payment_id_for_change, new_status.clone())
            .unwrap();
        let changed_value = new_value
            .clone()
            .payment_details()
            .into_iter()
            .find(|v| v.calculation().id() == payment_id_for_change)
            .unwrap();
        assert_eq!(changed_value.status(), new_status as u8);
    }
}
