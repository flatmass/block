use std::fmt::{Display, Formatter};
use std::str::FromStr;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::Error;

use super::lot::Bid;

thread_local! {
    static COST_REGEX: Regex = Regex::new(r"^\d+(.\d{1,2})?$").unwrap();
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone, Copy)]
pub struct Cost(u64);

impl From<Bid> for Cost {
    fn from(bid: Bid) -> Self {
        Cost(bid.value())
    }
}

impl From<Cost> for Bid {
    fn from(cost: Cost) -> Self {
        Bid::new(cost.0)
    }
}

impl From<Cost> for u64 {
    fn from(cost: Cost) -> Self {
        cost.0
    }
}

impl From<u64> for Cost {
    fn from(cost: u64) -> Self {
        Cost(cost)
    }
}

impl From<&Cost> for f64 {
    fn from(cost: &Cost) -> Self {
        (cost.0 as f64) / 100f64
    }
}

impl FromStr for Cost {
    type Err = Error;

    fn from_str(src: &str) -> Result<Self, Error> {
        COST_REGEX
            .with(|re| {
                re.captures(src)
                    .ok_or_else(|| Error::bad_price_format(src))
                    .and_then(|caps| {
                        caps[0]
                            .parse::<f64>()
                            .map(|v| (v * 100_f64).round() as u64)
                            .map_err(|_| Error::bad_price_format(src))
                    })
            })
            .map(Cost)
    }
}

impl Display for Cost {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", f64::from(self))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn de_cost() {
        let cost: Cost = serde_json::from_str("100").unwrap();
        let true_cost = Cost(100);
        assert_eq!(cost, true_cost)
    }

    #[test]
    fn ser_cost() {
        let cost = Cost(100);
        let cost_str = serde_json::to_string(&cost).unwrap();
        let true_str = String::from("100");
        assert_eq!(cost_str, true_str)
    }
}
