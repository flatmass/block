use std::convert::{TryFrom, TryInto};
use std::ops::Deref;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};

use blockp_core::blockchain::Transaction;
use blockp_core::crypto::Hash;
use blockp_core::crypto::HASH_SIZE;
use blockp_core::encoding::serialize::FromHex;

use crate::data::attachment::Sign;
use crate::data::classifier::Classifier;
use crate::data::conditions::{
    CheckResult, Conditions, ContractType, ExtraCondition, ObjectOwnership, TerminationCondition,
};
use crate::data::cost::Cost;
use crate::data::location::Location;
use crate::data::lot::{verify_lot_desc, verify_lot_name, Lot, LotId, LotStatus, SaleType};
use crate::data::member::MemberIdentity;
use crate::data::object::ObjectIdentity;
use crate::data::ownership::{Distribution, Ownership, OwnershipUnstructured};
use crate::data::time::{Duration, Specification, Term};
use crate::error::{Error, Result};

const HASH_SIZE_IN_HEX_FORMAT: usize = HASH_SIZE * 2;

pub struct TxList(pub Vec<String>);

pub type Lots = Vec<LotId>;

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct MemberInfo(#[serde(with = "serde_with::rust::display_fromstr")] pub MemberIdentity);

impl From<MemberInfo> for MemberIdentity {
    fn from(member: MemberInfo) -> MemberIdentity {
        member.0
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct ObjectInfo(#[serde(with = "serde_with::rust::display_fromstr")] pub ObjectIdentity);

impl From<ObjectInfo> for ObjectIdentity {
    fn from(object: ObjectInfo) -> ObjectIdentity {
        object.0
    }
}

impl From<ObjectIdentity> for ObjectInfo {
    fn from(object: ObjectIdentity) -> ObjectInfo {
        Self(object)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct LocationInfo(#[serde(with = "serde_with::rust::display_fromstr")] pub Location);

impl From<LocationInfo> for Location {
    fn from(location: LocationInfo) -> Location {
        location.0
    }
}

impl From<Location> for LocationInfo {
    fn from(location: Location) -> LocationInfo {
        LocationInfo(location)
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct ClassifierInfo(#[serde(with = "serde_with::rust::display_fromstr")] pub Classifier);

impl From<ClassifierInfo> for Classifier {
    fn from(classifier: ClassifierInfo) -> Classifier {
        classifier.0
    }
}

impl From<Classifier> for ClassifierInfo {
    fn from(classifier: Classifier) -> ClassifierInfo {
        ClassifierInfo(classifier)
    }
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LotInfo {
    name: String,
    desc: String,
    price: Cost,
    #[serde(deserialize_with = "serde_with::rust::display_fromstr::deserialize")]
    sale_type: SaleType,
    opening_time: DateTime<Utc>,
    closing_time: DateTime<Utc>,
    #[serde(skip_deserializing, default = "LotStatus::default")]
    status: LotStatus,
}

impl LotInfo {
    pub fn into_lot(&self) -> Result<Lot> {
        verify_lot_name(&self.name)?;
        verify_lot_desc(&self.desc)?;
        let price: Cost = self.price.into();
        Ok(Lot::new(
            &self.name,
            &self.desc,
            price.into(),
            self.sale_type as u8,
            self.opening_time,
            self.closing_time,
        ))
    }

    pub fn set_price(mut self, price: Cost) -> Self {
        self.price = price;
        self
    }

    pub fn set_status(mut self, status: LotStatus) -> Self {
        self.status = status;
        self
    }
}

impl From<&Lot> for LotInfo {
    fn from(lot: &Lot) -> Self {
        let price = Cost::from(lot.price());
        //TODO remove unwrap
        let sale_type = SaleType::try_from(lot.sale_type()).unwrap();
        let status = LotStatus::Undefined;
        Self {
            name: lot.name().to_owned(),
            desc: lot.desc().to_owned(),
            price,
            sale_type,
            opening_time: lot.opening_time(),
            closing_time: lot.closing_time(),
            status,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct TxHash(String);

impl From<&dyn Transaction> for TxHash {
    fn from(tx: &dyn Transaction) -> Self {
        TxHash(tx.hash().to_hex())
    }
}

impl From<Hash> for TxHash {
    fn from(tx_hash: Hash) -> Self {
        TxHash(tx_hash.to_hex())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase", tag = "representation")]
pub enum OwnershipInfo {
    Structured(StructuredOwnershipInfo),
    Unstructured(UnstructuredOwnershipInfo),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct StructuredOwnershipInfo {
    rightholder: MemberInfo,
    contract_type: ContractType,
    exclusive: bool,
    can_distribute: Distribution,
    location: Vec<LocationInfo>,
    classifiers: Vec<ClassifierInfo>,
    starting_time: DateTime<Utc>,
    expiration_time: Option<DateTime<Utc>>,
}

impl From<StructuredOwnershipInfo> for Ownership {
    fn from(info: StructuredOwnershipInfo) -> Self {
        Ownership::new(
            info.rightholder.into(),
            info.contract_type as u8,
            info.exclusive,
            info.can_distribute as u8,
            info.location.into_iter().map(Into::into).collect(),
            info.classifiers.into_iter().map(Into::into).collect(),
            info.starting_time.into(),
            info.expiration_time.map(Into::into),
        )
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct UnstructuredOwnershipInfo {
    data: Option<String>,
    rightholder: Option<MemberInfo>,
    exclusive: Option<bool>,
}

impl From<UnstructuredOwnershipInfo> for OwnershipUnstructured {
    fn from(info: UnstructuredOwnershipInfo) -> Self {
        OwnershipUnstructured::new(
            info.data.as_ref().map(|s| s.as_ref()).unwrap_or(""),
            info.rightholder.map(Into::into),
            info.exclusive,
        )
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ConditionsInfo {
    #[serde(with = "serde_with::rust::display_fromstr")]
    contract_type: ContractType,
    objects: Vec<ObjectOwnershipInfo>,
    payment_conditions: String,
    #[serde(
        deserialize_with = "serde_with::rust::default_on_null::deserialize",
        default
    )]
    payment_comment: String,
    termination_conditions: Vec<String>,
    contract_extras: Vec<String>,
}

impl From<ConditionsInfo> for Conditions {
    fn from(val: ConditionsInfo) -> Self {
        let objects: Vec<ObjectOwnership> = val.objects.into_iter().map(|val| val.into()).collect();
        let termination_conditions = val
            .termination_conditions
            .into_iter()
            .map(|val| TerminationCondition::new(val.as_str()))
            .collect();
        let contract_extras = val
            .contract_extras
            .into_iter()
            .map(|val| ExtraCondition::new(val.as_str()))
            .collect();
        Self::new(
            val.contract_type as u8,
            objects,
            &val.payment_conditions,
            &val.payment_comment,
            termination_conditions,
            contract_extras,
        )
    }
}

impl TryFrom<Conditions> for ConditionsInfo {
    type Error = Error;

    fn try_from(val: Conditions) -> Result<Self> {
        let objects: Vec<ObjectOwnershipInfo> = val
            .objects()
            .into_iter()
            .map(|val| val.try_into())
            .collect::<Result<Vec<ObjectOwnershipInfo>>>()?;
        let termination_conditions = val
            .termination_conditions()
            .into_iter()
            .map(|val| val.info().to_string())
            .collect();
        let contract_extras = val
            .contract_extras()
            .into_iter()
            .map(|val| val.info().to_string())
            .collect();
        Ok(Self {
            contract_type: ContractType::try_from(val.contract_type())
                .map_err(|e| Error::internal_bad_struct(&e.to_string()))?,
            objects,
            payment_conditions: val.payment_conditions().to_string(),
            payment_comment: val.payment_comment().to_string(),
            termination_conditions,
            contract_extras,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ObjectOwnershipInfo {
    #[serde(with = "serde_with::rust::display_fromstr")]
    object: ObjectIdentity,
    #[serde(deserialize_with = "serde_with::rust::default_on_null::deserialize")]
    contract_term: TermInfo,
    exclusive: bool,
    can_distribute: Distribution,
    #[serde(deserialize_with = "vec_default_on_null")]
    location: Vec<LocationInfo>,
    #[serde(deserialize_with = "vec_default_on_null")]
    classifiers: Vec<ClassifierInfo>,
}

fn vec_default_on_null<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or(vec![T::default()]))
}

impl From<ObjectOwnershipInfo> for ObjectOwnership {
    fn from(val: ObjectOwnershipInfo) -> Self {
        let location = if val.location.is_empty() {
            vec![Location::default()]
        } else {
            val.location.into_iter().map(Into::into).collect()
        };
        let classifiers = if val.classifiers.is_empty() {
            vec![Classifier::default()]
        } else {
            val.classifiers.into_iter().map(Into::into).collect()
        };
        ObjectOwnership::new(
            val.object,
            val.contract_term.into(),
            val.exclusive,
            val.can_distribute as u8,
            location,
            classifiers,
        )
    }
}

impl TryFrom<ObjectOwnership> for ObjectOwnershipInfo {
    type Error = Error;

    fn try_from(val: ObjectOwnership) -> Result<Self> {
        Ok(Self {
            object: val.object(),
            contract_term: val.contract_term().try_into()?,
            exclusive: val.exclusive(),
            can_distribute: Distribution::try_from(val.can_distribute())
                .map_err(|e| Error::internal_bad_struct(&e.to_string()))?,
            location: val.location().into_iter().map(|val| val.into()).collect(),
            classifiers: val
                .classifiers()
                .into_iter()
                .map(|val| val.into())
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "lowercase", tag = "specification")]
pub enum TermInfo {
    For {
        #[serde(with = "serde_with::rust::display_fromstr")]
        duration: Duration,
    },
    To {
        date: DateTime<Utc>,
    },
    Until {
        date: DateTime<Utc>,
    },
    Forever,
}

impl From<TermInfo> for Term {
    fn from(t: TermInfo) -> Self {
        match t {
            TermInfo::For { duration } => Self::new(Specification::For as u8, Some(duration), None),
            TermInfo::To { date } => Self::new(Specification::To as u8, None, Some(date)),
            TermInfo::Until { date } => Self::new(Specification::Until as u8, None, Some(date)),
            TermInfo::Forever => Self::new(Specification::Forever as u8, None, None),
        }
    }
}

impl TryFrom<Term> for TermInfo {
    type Error = Error;
    fn try_from(t: Term) -> Result<Self> {
        let spec = Specification::try_from(t.specification())
            .map_err(|e| Error::internal_bad_struct(&e.to_string()))?;
        let term_info = match spec {
            Specification::For => TermInfo::For {
                duration: t
                    .duration()
                    .ok_or_else(|| Error::unexpected_param_value("duration"))?,
            },
            Specification::To => TermInfo::To {
                date: t
                    .date()
                    .ok_or_else(|| Error::unexpected_param_value("date"))?,
            },
            Specification::Until => TermInfo::Until {
                date: t
                    .date()
                    .ok_or_else(|| Error::unexpected_param_value("date"))?,
            },
            Specification::Forever => TermInfo::Forever,
        };
        Ok(term_info)
    }
}

impl Default for TermInfo {
    fn default() -> Self {
        Self::Forever
    }
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct SignInfo(#[serde(with = "serde_with::rust::display_fromstr")] pub Sign);

impl From<SignInfo> for Sign {
    fn from(sign: SignInfo) -> Sign {
        sign.0
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckResultInfo {
    Ok = 1,
    Unknown = 0,
    Error = -1,
}

impl From<i8> for CheckResultInfo {
    fn from(res: i8) -> Self {
        if res > 0 {
            Self::Ok
        } else if res < 0 {
            Self::Error
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CheckInfo {
    result: CheckResultInfo,
    description: String,
}

impl From<CheckResult> for CheckInfo {
    fn from(res: CheckResult) -> Self {
        Self {
            result: CheckResultInfo::from(res.result()),
            description: res.desc().to_owned(),
        }
    }
}

impl From<CheckInfo> for CheckResult {
    fn from(info: CheckInfo) -> Self {
        Self::new(info.result as i8, info.description.as_str())
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct HashInfo(#[serde(deserialize_with = "deserialize_hash_with_length_check")] pub Hash);

impl Deref for HashInfo {
    type Target = Hash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Hash> for HashInfo {
    fn as_ref(&self) -> &Hash {
        &self.0
    }
}

impl FromStr for HashInfo {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.len() != HASH_SIZE_IN_HEX_FORMAT {
            Err(Error::invalid_string_length(HASH_SIZE_IN_HEX_FORMAT))
        } else {
            Ok(Self(Hash::from_str(s)?))
        }
    }
}

pub fn deserialize_hash_with_length_check<'de, D>(
    deserializer: D,
) -> std::result::Result<Hash, D::Error>
where
    D: Deserializer<'de>,
{
    struct HexVisitor;

    impl<'v> Visitor<'v> for HexVisitor {
        type Value = Hash;
        fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                fmt,
                "expecting string with length = {}",
                HASH_SIZE_IN_HEX_FORMAT
            )
        }
        fn visit_str<E>(self, s: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if s.len() != HASH_SIZE_IN_HEX_FORMAT {
                Err(serde::de::Error::custom(format!(
                    "expecting string with length = {}",
                    HASH_SIZE_IN_HEX_FORMAT
                )))?
            }
            Hash::from_hex(s).map_err(|_| serde::de::Error::custom("Invalid hex"))
        }
    }
    deserializer.deserialize_str(HexVisitor)
}

impl From<HashInfo> for Hash {
    fn from(val: HashInfo) -> Self {
        val.0
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ContractInfo {
    pub buyer: MemberInfo,
    pub seller: MemberInfo,
    pub price: u64,
    pub conditions: ConditionsInfo,
    pub status: String,
    pub deed_tx_hash: Option<Hash>,
    pub application_tx_hash: Option<Hash>,
    pub stored_docs: Vec<Hash>,
    pub reference_number: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct LotInfoWithObjects {
    #[serde(flatten)]
    pub lot: LotInfo,
    pub objects: Vec<ObjectInfo>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ObjectData(String);

impl From<String> for ObjectData {
    fn from(data: String) -> Self {
        Self(data)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use std::str::FromStr;

    use super::*;

    pub fn new_lot_info() -> LotInfo {
        // {
        //     "name": "My Lot 1",
        //     "desc": "Explicit lot description",
        //     "price": "50000",
        //     "sale_type": "auction",
        //     "opening_time": "2020-12-10T02:00:53+00:00",
        //     "closing_time": "2020-12-31T05:00:53+00:00",
        //     "status": "undefined"
        // }
        LotInfo {
            name: "My Lot 1".to_string(),
            desc: "Explicit lot description".to_string(),
            price: Cost::from(5000000),
            sale_type: SaleType::Auction,
            opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
            closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
            status: Default::default(),
        }
    }

    pub fn new_object_ownership_info() -> ObjectOwnershipInfo {
        // {
        //     "object": "trademark::123451",
        //     "contract_term": { "specification": "forever" },
        //     "exclusive": false,
        //     "can_distribute": "unable",
        //     "location": ["oktmo::45379000"],
        //     "classifier": ["mktu::8", "mktu::13"]
        // }
        ObjectOwnershipInfo {
            object: ObjectIdentity::from_str("trademark::123451").unwrap(),
            contract_term: Default::default(),
            exclusive: false,
            can_distribute: Distribution::Unable,
            location: vec![LocationInfo(Location::from_str("oktmo::45379000").unwrap())],
            classifiers: vec![
                ClassifierInfo(Classifier::from_str("mktu::8").unwrap()),
                ClassifierInfo(Classifier::from_str("mktu::13").unwrap()),
            ],
        }
    }

    pub fn new_conditions_info() -> ConditionsInfo {
        // {
        //     "contract_type": "license",
        //     "objects": {
        //         "object": "trademark::123451",
        //         "contract_term": { "specification": "forever" },
        //         "exclusive": false,
        //         "can_distribute": "unable",
        //         "location": "oktmo::45379000",
        //         "classifier": ["mktu::8", "mktu::13"],
        //     },
        //     "payment_conditions": "Condition desc text",
        //     "payment_comment": null,
        //     "termination_conditions": ["Term cond 1", "Term cond 2"],
        //     "contract_extras": ["Extra comment"],
        // }
        ConditionsInfo {
            contract_type: ContractType::License,
            objects: vec![new_object_ownership_info()],
            payment_conditions: "Condition desc text".to_string(),
            payment_comment: "".to_string(),
            termination_conditions: vec!["Term cond 1".to_string(), "Term cond 2".to_string()],
            contract_extras: vec!["Extra comment".to_string()],
        }
    }

    pub fn new_check_info() -> CheckInfo {
        // {
        //     "result": "ok",
        //     "description": "good"
        // }
        CheckInfo {
            result: CheckResultInfo::Ok,
            description: String::from("good"),
        }
    }

    pub fn new_structured_ownership_info() -> StructuredOwnershipInfo {
        // {
        //     "rightholder": "ogrn::5068681643685",
        //     "contract_type": "license",
        //     "exclusive": true,
        //     "can_distribute": "able",
        //     "location": [ "oktmo::45379000" ],
        //     "classifiers": [ "mktu::8" ],
        //     "starting_time": "2020-06-01T00:00:00Z",
        //     "expiration_time": "2021-06-01T00:00:00Z"
        // }
        StructuredOwnershipInfo {
            rightholder: MemberInfo(MemberIdentity::from_str("ogrn::5068681643685").unwrap()),
            contract_type: ContractType::License,
            exclusive: true,
            can_distribute: Distribution::Able,
            location: vec![LocationInfo(Location::from_str("oktmo::45379000").unwrap())],
            classifiers: vec![ClassifierInfo(Classifier::from_str("mktu::8").unwrap())],
            starting_time: DateTime::<Utc>::from_str("2020-06-01T00:00:00Z").unwrap(),
            expiration_time: Some(DateTime::<Utc>::from_str("2021-06-01T00:00:00Z").unwrap()),
        }
    }

    #[test]
    fn de_ownership_info_structured() {
        let object_json = r#"{
            "representation": "structured",
            "rightholder": "ogrn::5068681643685",
            "contract_type": "license",
            "exclusive": true,
            "can_distribute": "able",
            "location": [ "oktmo::45379000" ],
            "classifiers": [ "mktu::8" ],
            "starting_time": "2020-06-01T00:00:00Z",
            "expiration_time": "2021-06-01T00:00:00Z"
        }"#;
        let expected = OwnershipInfo::Structured(new_structured_ownership_info());
        let deserialized: OwnershipInfo = serde_json::from_str(object_json).unwrap();
        assert_eq!(deserialized, expected)
    }

    #[test]
    fn de_ownership_info_unstructured() {
        let object_json = r#"{
            "representation": "unstructured",
            "rightholder": "ogrn::5068681643685"
        }"#;
        let expected = OwnershipInfo::Unstructured(UnstructuredOwnershipInfo {
            rightholder: Some(MemberInfo(
                MemberIdentity::from_str("ogrn::5068681643685").unwrap(),
            )),
            data: None,
            exclusive: None,
        });
        let deserialized: OwnershipInfo = serde_json::from_str(object_json).unwrap();
        assert_eq!(deserialized, expected)
    }

    #[test]
    fn de_object_info() {
        use std::str::FromStr;

        let object_json = r#""trademark::123""#;
        let parced: ObjectInfo =
            serde_json::from_str(object_json).expect("Unable to parse ObjectInfo");
        let expected = ObjectInfo(ObjectIdentity::from_str("trademark::123").unwrap());
        assert_eq!(parced, expected);
    }

    #[test]
    fn ser_object_info() {
        use std::str::FromStr;

        let data = ObjectInfo(ObjectIdentity::from_str("trademark::123").unwrap());
        let result = serde_json::to_string(&data).expect("Unable to serialize ObjectInfo");
        assert_eq!(result, r#""trademark::123""#);
    }

    #[test]
    fn de_member_info() {
        use std::str::FromStr;

        let member_ogrn_json = r#""ogrn::1053600591197""#;
        let parced_ogrn: MemberInfo =
            serde_json::from_str(member_ogrn_json).expect("Unable to parse MemberInfo");
        let expected_ogrn = MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap());
        assert_eq!(parced_ogrn, expected_ogrn);

        let member_snils_json = r#""snils::02583651862""#;
        let parced_snils: MemberInfo =
            serde_json::from_str(member_snils_json).expect("Unable to parse MemberInfo");
        let expected_snils = MemberInfo(MemberIdentity::from_str("snils::02583651862").unwrap());
        assert_eq!(parced_snils, expected_snils);
    }

    #[test]
    fn ser_member_info() {
        use std::str::FromStr;

        let ogrn_data = MemberInfo(MemberIdentity::from_str("ogrn::1053600591197").unwrap());
        let ogrn_result =
            serde_json::to_string(&ogrn_data).expect("Unable to serialize MemberInfo");
        assert_eq!(ogrn_result, r#""ogrn::1053600591197""#);

        let snils_data = MemberInfo(MemberIdentity::from_str("snils::02583651862").unwrap());
        let snils_result =
            serde_json::to_string(&snils_data).expect("Unable to serialize MemberInfo");
        assert_eq!(snils_result, r#""snils::02583651862""#);
    }

    #[test]
    fn de_term_info() {
        let json_for = r#"
        {
            "specification" : "for",
            "duration" : "10:5"
        }"#;
        let term_for: TermInfo = serde_json::from_str(json_for).unwrap();
        let true_term_for = TermInfo::For {
            duration: Duration::new(10, 5),
        };
        assert_eq!(term_for, true_term_for);

        let json_to = r#"
        {
            "specification" : "to",
            "date" : "2025-04-28T12:32:30+00:00"
        }"#;
        let term_to: TermInfo = serde_json::from_str(json_to).unwrap();
        let true_term_to = TermInfo::To {
            date: DateTime::<Utc>::from_str("2025-04-28T12:32:30+00:00").unwrap(),
        };
        assert_eq!(term_to, true_term_to);

        let json_until = r#"
        {
            "specification" : "until",
            "date" : "2025-04-28T12:32:30+00:00"
        }"#;
        let term_until: TermInfo = serde_json::from_str(json_until).unwrap();
        let true_term_until = TermInfo::Until {
            date: DateTime::<Utc>::from_str("2025-04-28T12:32:30+00:00").unwrap(),
        };
        assert_eq!(term_until, true_term_until);

        let json_forever = r#"
        {
            "specification" : "forever"
        }"#;
        let term_forever: TermInfo = serde_json::from_str(json_forever).unwrap();
        let true_term_forever = TermInfo::Forever;
        assert_eq!(term_forever, true_term_forever);
    }

    #[test]
    fn de_object_ownership_info() {
        let json = r#"
        {
            "object": "trademark::123451",
            "contract_term": { "specification": "forever" },
            "exclusive": false,
            "can_distribute": "unable",
            "location": ["oktmo::45379000"],
            "classifiers": ["mktu::8", "mktu::13"]
        }"#;

        let true_val = ObjectOwnershipInfo {
            object: ObjectIdentity::from_str("trademark::123451").unwrap(),
            contract_term: Default::default(),
            exclusive: false,
            can_distribute: Distribution::Unable,
            location: vec![LocationInfo(Location::from_str("oktmo::45379000").unwrap())],
            classifiers: vec![
                ClassifierInfo(Classifier::from_str("mktu::8").unwrap()),
                ClassifierInfo(Classifier::from_str("mktu::13").unwrap()),
            ],
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_object_ownership_info_with_nulls() {
        let json = r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": null
        }"#;

        let true_val = ObjectOwnershipInfo {
            object: ObjectIdentity::from_str("trademark::123451").unwrap(),
            contract_term: Default::default(),
            exclusive: false,
            can_distribute: Distribution::Unable,
            location: vec![LocationInfo(Location::default())],
            classifiers: vec![ClassifierInfo(Classifier::default())],
        };
        let val: ObjectOwnershipInfo = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_object_ownership_info_bad_value() {
        let json = vec![
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": "bad_value",
            "classifiers": null
        }"#,
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": "bad_value"
        }"#,
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": 1,
            "classifiers": null
        }"#,
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": 1
        }"#,
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": {"bad_parametr": "bad value"}
        }"#,
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": {"bad_parametr": "bad value"},
            "classifiers": null
        }"#,
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": [""],
            "classifiers": null
        }"#,
            r#"
        {
            "object": "trademark::123451",
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": [""]
        }"#,
        ];
        let check = json
            .into_iter()
            .map(|json| serde_json::from_str::<ObjectOwnershipInfo>(json))
            .all(|value| value.is_err());
        assert!(check);
    }

    #[test]
    fn de_conditions_info() {
        let json = r#"
        {
            "contract_type": "license",
            "objects": [{
                "object": "trademark::123451",
                "contract_term": { "specification": "forever" },
                "exclusive": false,
                "can_distribute": "unable",
                "location": ["oktmo::45379000"],
                "classifiers": ["mktu::8", "mktu::13"]
            }],
            "payment_conditions": "Condition desc text",
            "payment_comment": "test text",
            "termination_conditions": ["Term cond 1", "Term cond 2"],
            "contract_extras": ["Extra comment"]
        }"#;

        let true_val = ConditionsInfo {
            contract_type: ContractType::License,
            objects: vec![new_object_ownership_info()],
            payment_conditions: "Condition desc text".to_string(),
            payment_comment: "test text".to_string(),
            termination_conditions: vec!["Term cond 1".to_string(), "Term cond 2".to_string()],
            contract_extras: vec!["Extra comment".to_string()],
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_conditions_info_with_nulls() {
        let json = r#"
        {
            "contract_type": "license",
            "objects": [{
                "object": "trademark::123451",
                "contract_term": { "specification": "forever" },
                "exclusive": false,
                "can_distribute": "unable",
                "location": ["oktmo::45379000"],
                "classifiers": ["mktu::8", "mktu::13"]
            }],
            "payment_conditions": "Condition desc text",
            "payment_comment": null,
            "termination_conditions": ["Term cond 1", "Term cond 2"],
            "contract_extras": ["Extra comment"]
        }"#;

        let true_val = ConditionsInfo {
            contract_type: ContractType::License,
            objects: vec![new_object_ownership_info()],
            payment_conditions: "Condition desc text".to_string(),
            payment_comment: Default::default(),
            termination_conditions: vec!["Term cond 1".to_string(), "Term cond 2".to_string()],
            contract_extras: vec!["Extra comment".to_string()],
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_check_info() {
        let json = r#"
        {
            "result": "ok",
            "description": "good"
        }"#;

        let true_val = CheckInfo {
            result: CheckResultInfo::Ok,
            description: String::from("good"),
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_hash_info() {
        let json = r#"
        {
            "tx_hash": "d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"
        }"#;

        #[derive(Debug, Deserialize, Eq, PartialEq)]
        struct HashInfoJson {
            tx_hash: HashInfo,
        }

        let true_val = HashInfoJson {
            tx_hash: HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            ),
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);

        let json = r#"
        {
            "tx_hash": ["d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad"]
        }"#;

        #[derive(Debug, Deserialize, Eq, PartialEq)]
        struct HashInfoArrayJson {
            tx_hash: Vec<HashInfo>,
        }

        let true_val = HashInfoArrayJson {
            tx_hash: vec![HashInfo(
                Hash::from_str("d731bcdfcb3a0dc8dc91b492c7756e16b40867b0fb0df960e3c37bd23751f1ad")
                    .unwrap(),
            )],
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_lot_info() {
        let json = r#"{
                "name": "My Lot 1",
                "desc": "Explicit lot description",
                "price": 50000,
                "sale_type": "auction",
                "opening_time": "2020-12-10T02:00:53+00:00",
                "closing_time": "2020-12-31T05:00:53+00:00",
                "status": "undefined"
            }"#;
        let true_val = LotInfo {
            name: "My Lot 1".to_string(),
            desc: "Explicit lot description".to_string(),
            price: Cost::from(50000),
            sale_type: SaleType::Auction,
            opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
            closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
            status: Default::default(),
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn se_lot_info() {
        let true_json = r#"{"name":"My Lot 1","desc":"Explicit lot description","price":50000,"sale_type":"auction","opening_time":"2020-12-10T02:00:53Z","closing_time":"2020-12-31T05:00:53Z","status":"undefined"}"#;
        let val = LotInfo {
            name: "My Lot 1".to_string(),
            desc: "Explicit lot description".to_string(),
            price: Cost::from(50000),
            sale_type: SaleType::Auction,
            opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
            closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
            status: Default::default(),
        };
        let val = serde_json::to_string(&val).unwrap();
        assert_eq!(val, true_json);
    }

    #[test]
    fn de_lot_info_with_objects() {
        let json = r#"{
                "name": "My Lot 1",
                "desc": "Explicit lot description",
                "price": 50000,
                "sale_type": "auction",
                "opening_time": "2020-12-10T02:00:53+00:00",
                "closing_time": "2020-12-31T05:00:53+00:00",
                "status": "undefined",
                "objects": ["trademark::123"]
            }"#;
        let true_val = LotInfoWithObjects {
            lot: LotInfo {
                name: "My Lot 1".to_string(),
                desc: "Explicit lot description".to_string(),
                price: Cost::from(50000),
                sale_type: SaleType::Auction,
                opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
                closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
                status: Default::default(),
            },
            objects: vec![ObjectInfo(
                ObjectIdentity::from_str("trademark::123").unwrap(),
            )],
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn se_lot_info_with_objects() {
        let true_json = r#"{"name":"My Lot 1","desc":"Explicit lot description","price":50000,"sale_type":"auction","opening_time":"2020-12-10T02:00:53Z","closing_time":"2020-12-31T05:00:53Z","status":"undefined","objects":["trademark::123"]}"#;
        let val = LotInfoWithObjects {
            lot: LotInfo {
                name: "My Lot 1".to_string(),
                desc: "Explicit lot description".to_string(),
                price: Cost::from(50000),
                sale_type: SaleType::Auction,
                opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
                closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
                status: Default::default(),
            },
            objects: vec![ObjectInfo(
                ObjectIdentity::from_str("trademark::123").unwrap(),
            )],
        };
        let val = serde_json::to_string(&val).unwrap();
        assert_eq!(val, true_json);
    }
}
