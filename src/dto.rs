use std::convert::{TryFrom, TryInto};
use std::ops::Deref;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize};

use blockp_core::blockchain::Transaction;
use blockp_core::crypto::Hash;
use blockp_core::crypto::HASH_SIZE;
use blockp_core::encoding::serialize::FromHex;

use crate::data::attachment::{
    Attachment, AttachmentMetadata, AttachmentMetadataWithHash, AttachmentType, DocumentId, Sign,
};
use crate::data::classifier::Classifier;
use crate::data::conditions::{
    CheckResult, Conditions, ContractType, ExtraCondition, ObjectOwnership, TerminationCondition,
};
use crate::data::contract::{ContractId, RequestConfirm};
use crate::data::cost::Cost;
use crate::data::location::Location;
use crate::data::lot::{Lot, LotId, LotStatus, SaleType};
#[cfg(feature = "internal_api")]
use crate::data::member::MemberEsiaToken;
use crate::data::member::MemberIdentity;
use crate::data::object::ObjectIdentity;
use crate::data::ownership::{Distribution, Ownership, OwnershipUnstructured, Rights};
use crate::data::payment::{Calculation, PaymentDetail, PaymentStatus};
use crate::data::time::{Duration, Specification, Term};
use crate::error::{Error, Result};

const HASH_SIZE_IN_HEX_FORMAT: usize = HASH_SIZE * 2;

pub struct TxList(pub Vec<String>);

pub type Lots = Vec<LotId>;

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct MemberInfo {
    class: u8,
    number: String,
}

impl<'de> Deserialize<'de> for MemberInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct UncheckedMemberInfo {
            class: u8,
            number: String,
        }

        impl UncheckedMemberInfo {
            fn is_valid(self) -> Result<MemberInfo> {
                let tmp = MemberInfo {
                    class: self.class,
                    number: self.number,
                };
                let mem_iden = MemberIdentity::from(tmp.clone());
                if mem_iden.is_valid() {
                    Ok(tmp)
                } else {
                    Error::bad_member_format(&mem_iden.to_string()).ok()
                }
            }
        }

        UncheckedMemberInfo::deserialize(deserializer)?
            .is_valid()
            .map_err(serde::de::Error::custom)
    }
}

impl From<MemberInfo> for MemberIdentity {
    fn from(v: MemberInfo) -> MemberIdentity {
        MemberIdentity::new(v.class, &v.number)
    }
}

impl From<MemberIdentity> for MemberInfo {
    fn from(v: MemberIdentity) -> MemberInfo {
        MemberInfo {
            class: v.class(),
            number: v.number().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct ObjectIdentityDto {
    class: u8,
    reg_number: String,
}

impl<'de> Deserialize<'de> for ObjectIdentityDto {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct UncheckedObjectIdentityDto {
            class: u8,
            reg_number: String,
        }

        impl UncheckedObjectIdentityDto {
            fn is_valid(self) -> Result<ObjectIdentityDto> {
                let tmp = ObjectIdentityDto {
                    class: self.class,
                    reg_number: self.reg_number,
                };
                let mem_iden = ObjectIdentity::from(tmp.clone());
                if mem_iden.is_valid() {
                    Ok(tmp)
                } else {
                    Error::bad_object_format(&mem_iden.to_string(), "invalid number").ok()
                }
            }
        }

        UncheckedObjectIdentityDto::deserialize(deserializer)?
            .is_valid()
            .map_err(serde::de::Error::custom)
    }
}

impl FromStr for ObjectIdentityDto {
    type Err = Error;

    fn from_str(object: &str) -> Result<Self> {
        let tmp = ObjectIdentity::from_str(object)?;
        Ok(tmp.into())
    }
}

// impl Display for ObjectInfo {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", &self.0)
//     }
// }

impl From<ObjectIdentityDto> for ObjectIdentity {
    fn from(object: ObjectIdentityDto) -> ObjectIdentity {
        ObjectIdentity::new(object.class, &object.reg_number)
    }
}

impl From<ObjectIdentity> for ObjectIdentityDto {
    fn from(object: ObjectIdentity) -> ObjectIdentityDto {
        Self {
            class: object.class(),
            reg_number: object.reg_number().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Eq, PartialEq, Clone)]
pub struct LocationInfo {
    registry: u8,
    code: u64,
    desc: String,
}

impl<'de> Deserialize<'de> for LocationInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct UncheckedLocationInfo {
            registry: u8,
            code: u64,
            #[serde(default)]
            desc: String,
        }

        impl UncheckedLocationInfo {
            fn is_valid(self) -> Result<LocationInfo> {
                let tmp = LocationInfo {
                    registry: self.registry,
                    code: self.code,
                    desc: self.desc,
                };
                let mem_iden = Location::from(tmp.clone());
                if mem_iden.is_valid() {
                    Ok(tmp)
                } else {
                    Err(Error::bad_location(&mem_iden.to_string()))
                }
            }
        }

        UncheckedLocationInfo::deserialize(deserializer)?
            .is_valid()
            .map_err(serde::de::Error::custom)
    }
}

impl From<LocationInfo> for Location {
    fn from(location: LocationInfo) -> Location {
        Location::new(location.registry, location.code, &location.desc)
    }
}

impl From<Location> for LocationInfo {
    fn from(location: Location) -> LocationInfo {
        LocationInfo {
            registry: location.registry(),
            code: location.code(),
            desc: location.desc().to_string(),
        }
    }
}

impl Default for LocationInfo {
    fn default() -> Self {
        Location::default().into()
    }
}

#[derive(Debug, Default, Serialize, Eq, PartialEq, Clone)]
pub struct ClassifierInfo {
    registry: u8,
    value: String,
    desc: String,
}

impl<'de> Deserialize<'de> for ClassifierInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct UncheckedClassifierInfo {
            registry: u8,
            value: String,
            #[serde(default)]
            desc: String,
        }

        impl UncheckedClassifierInfo {
            fn is_valid(self) -> Result<ClassifierInfo> {
                let tmp = ClassifierInfo {
                    registry: self.registry,
                    value: self.value,
                    desc: self.desc,
                };
                let mem_iden = Classifier::from(tmp.clone());
                mem_iden.is_valid()?;
                Ok(tmp)
            }
        }

        UncheckedClassifierInfo::deserialize(deserializer)?
            .is_valid()
            .map_err(serde::de::Error::custom)
    }
}

impl From<ClassifierInfo> for Classifier {
    fn from(classifier: ClassifierInfo) -> Classifier {
        Classifier::new(classifier.registry, &classifier.value, &classifier.desc)
    }
}

impl From<Classifier> for ClassifierInfo {
    fn from(classifier: Classifier) -> ClassifierInfo {
        ClassifierInfo {
            registry: classifier.registry(),
            value: classifier.value().to_string(),
            desc: classifier.desc().to_string(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize)]
pub struct LotInfo {
    name: String,
    desc: String,
    // https://aj.srvdev.ru/browse/FIPSOP-963 РБД. Смена механизма установки типа продажи лота
    // sale_type: Option<SaleType>,
    price: Cost,
    opening_time: DateTime<Utc>,
    closing_time: DateTime<Utc>,
}

impl LotInfo {
    pub fn into_lot(&self, seller: MemberIdentity, sale_type: SaleType) -> Result<Lot> {
        let price: Cost = self.price.into();
        Ok(Lot::new(
            &self.name,
            &self.desc,
            seller,
            price.into(),
            sale_type as u8,
            self.opening_time,
            self.closing_time,
        ))
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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

impl StructuredOwnershipInfo {
    pub fn from_rights(rights: Rights, rightholder_id: MemberIdentity) -> Result<Self> {
        Ok(StructuredOwnershipInfo {
            rightholder: rightholder_id.into(),
            contract_type: ContractType::try_from(rights.contract_type())
                .map_err(|e| Error::internal_bad_struct(&e.to_string()))?,
            exclusive: rights.is_exclusive(),
            can_distribute: Distribution::Able,
            location: rights.location().into_iter().map(Into::into).collect(),
            classifiers: rights.classifiers().into_iter().map(Into::into).collect(),
            starting_time: rights.starting_time(),
            expiration_time: rights.expiration_time(),
        })
    }
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
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

impl From<OwnershipUnstructured> for UnstructuredOwnershipInfo {
    fn from(v: OwnershipUnstructured) -> Self {
        UnstructuredOwnershipInfo {
            data: Some(v.data().to_owned()),
            rightholder: v.rightholder().map(Into::into),
            exclusive: v.exclusive().map(Into::into),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ConditionsInfo {
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub contract_type: ContractType,
    pub objects: Vec<ObjectOwnershipInfo>,
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
    object: ObjectIdentityDto,
    #[serde(deserialize_with = "serde_with::rust::default_on_null::deserialize")]
    contract_term: TermInfo,
    exclusive: bool,
    can_distribute: Distribution,
    #[serde(deserialize_with = "vec_default_on_null")]
    location: Vec<LocationInfo>,
    #[serde(deserialize_with = "vec_default_on_null")]
    classifiers: Vec<ClassifierInfo>,
}

impl ObjectOwnershipInfo {
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }
}

fn vec_default_on_null<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::deserialize(deserializer)?.unwrap_or_else(|| vec![T::default()]))
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
            val.object.into(),
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
            object: val.object().into(),
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

impl From<Sign> for SignInfo {
    fn from(sign: Sign) -> SignInfo {
        SignInfo(sign)
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
pub struct ContractDocuments {
    pub deed_file: Option<AttachmentMetadataWithHashDto>,
    pub application_file: Option<AttachmentMetadataWithHashDto>,
    pub other_files: Vec<AttachmentMetadataWithHashDto>,
    pub notification_files: Vec<AttachmentMetadataWithHashDto>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ContractInfo {
    pub buyer: MemberInfo,
    pub seller: MemberInfo,
    pub price: u64,
    pub conditions: ConditionsInfo,
    pub status: String,
    pub documents: ContractDocuments,
    pub reference_number: Option<String>,
    pub calculations: Vec<CalculationWithPaymentDetailInfo>,
    pub is_undefined: bool,
    pub contract_correspondence: Option<String>,
    pub objects_correspondence: Option<String>,
}

#[derive(Serialize, Debug, Eq, PartialEq)]
pub struct LotInfoWithObjects {
    pub name: String,
    pub desc: String,
    pub seller: MemberInfo,
    pub price: Cost,
    pub sale_type: SaleType,
    pub opening_time: DateTime<Utc>,
    pub closing_time: DateTime<Utc>,
    pub status: LotStatus,
    pub is_undefined: bool,
    pub conditions: ConditionsInfo,
    pub calculations: Vec<CalculationInfo>,
    pub reference_number: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ObjectParticipates {
    pub lots: Vec<LotId>,
    pub contracts: Vec<ContractId>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
pub struct CalculationInfo {
    pub id: String,
    pub data: String,
    #[serde(skip_deserializing, default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
}

impl CalculationInfo {
    pub fn is_valid(&self) -> Result<()> {
        if self.id.is_empty() {
            Error::empty_param("id").ok()?
        }
        if self.data.is_empty() {
            Error::empty_param("data").ok()?
        }
        if self.id.len() > 256 {
            Error::too_long_param("id").ok()?
        }
        Ok(())
    }
}

impl From<CalculationInfo> for Calculation {
    fn from(v: CalculationInfo) -> Self {
        Calculation::new(&v.id, &v.data, v.timestamp)
    }
}

impl From<Calculation> for CalculationInfo {
    fn from(v: Calculation) -> Self {
        CalculationInfo {
            id: v.id().to_string(),
            data: v.data().to_string(),
            timestamp: v.timestamp(),
        }
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct PaymentDetailsInfo {
    pub calculation: CalculationInfo,
    payment_detail: String,
}

impl PaymentDetailsInfo {
    pub fn is_valid(&self) -> Result<()> {
        self.calculation.is_valid()?;
        if self.payment_detail.is_empty() {
            Error::empty_param("payment_detail").ok()?
        }
        if self.payment_detail.len() > 256 {
            Error::too_long_param("payment_detail").ok()?
        }
        Ok(())
    }
}

impl From<PaymentDetailsInfo> for PaymentDetail {
    fn from(v: PaymentDetailsInfo) -> Self {
        PaymentDetail::new(
            v.calculation.into(),
            &v.payment_detail,
            PaymentStatus::NotPaid as u8,
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct CalculationWithPaymentDetailInfo {
    calculation: CalculationInfo,
    payment_detail: Option<PaymentDetailInfo>,
}

impl TryFrom<PaymentDetail> for CalculationWithPaymentDetailInfo {
    type Error = Error;

    fn try_from(v: PaymentDetail) -> Result<Self> {
        let payment_detail = if v.payment_detail().is_empty() {
            None
        } else {
            Some(PaymentDetailInfo {
                payment_detail: v.payment_detail().to_string(),
                status: PaymentStatus::try_from(v.status())?,
            })
        };
        Ok(CalculationWithPaymentDetailInfo {
            calculation: v.calculation().into(),
            payment_detail,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct PaymentDetailInfo {
    payment_detail: String,
    status: PaymentStatus,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PaginationPage<V, K> {
    pub data: Vec<V>,
    pub from: K,
    pub limit: usize,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ObjectInformationDto {
    pub object: ObjectIdentityDto,
    pub data: String,
    pub ownership: Vec<StructuredOwnershipInfo>,
    pub unstructured_ownership: Vec<UnstructuredOwnershipInfo>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ConfirmDto {
    pub buyer: bool,
    pub seller: bool,
}

impl From<RequestConfirm> for ConfirmDto {
    fn from(v: RequestConfirm) -> Self {
        Self {
            buyer: *v.is_buyer(),
            seller: *v.is_seller(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RequestConfirmDto {
    pub status: ConfirmDto,
    pub status_gone: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct AttachmentMetadataDto {
    name: String,
    description: Option<String>,
    file_type: AttachmentType,
    timestamp: DateTime<Utc>,
}

impl TryFrom<AttachmentMetadata> for AttachmentMetadataDto {
    type Error = Error;

    fn try_from(v: AttachmentMetadata) -> Result<Self> {
        Ok(Self {
            name: v.name().to_owned(),
            description: v.description(),
            file_type: AttachmentType::try_from(v.file_type())?,
            timestamp: v.timestamp(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct AttachmentMetadataWithHashDto {
    metadata: AttachmentMetadataDto,
    tx_hash: DocumentId,
}

impl AttachmentMetadataWithHashDto {
    pub fn new(tx_hash: DocumentId, metadata: AttachmentMetadataDto) -> Self {
        AttachmentMetadataWithHashDto { metadata, tx_hash }
    }
}

impl TryFrom<AttachmentMetadataWithHash> for AttachmentMetadataWithHashDto {
    type Error = Error;

    fn try_from(v: AttachmentMetadataWithHash) -> Result<Self> {
        Ok(Self {
            metadata: v.metadata().try_into()?,
            tx_hash: v.tx_hash().to_owned(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct AttachmentDto {
    metadata: AttachmentMetadataDto,
    data: String,
    sign: Option<SignInfo>,
    pub buyer_sign: Option<SignInfo>,
    pub seller_sign: Option<SignInfo>,
}

// Use only for attachments/doc with single sign, for example deed and application needed in two sign from buyer and from seller.
impl TryFrom<Attachment> for AttachmentDto {
    type Error = Error;
    fn try_from(v: Attachment) -> Result<Self> {
        Ok(AttachmentDto {
            metadata: v.metadata().try_into()?,
            data: base64::encode(v.data()),
            sign: v.sign().map(Into::into),
            buyer_sign: None,
            seller_sign: None,
        })
    }
}

// impl TryFrom<(Attachment, Sign)> for AttachmentDto {
//     type Error = Error;
//     fn try_from(v: (Attachment, Sign)) -> Result<Self> {
//         Ok(AttachmentDto {
//             metadata: v.0.metadata().try_into()?,
//             data: base64::encode(v.0.data()),
//             sign: Some(v.1.into()),
//         })
//     }
// }
//
// impl TryFrom<(Attachment, Option<Sign>)> for AttachmentDto {
//     type Error = Error;
//     fn try_from(v: (Attachment, Option<Sign>)) -> Result<Self> {
//         Ok(AttachmentDto {
//             metadata: v.0.metadata().try_into()?,
//             data: base64::encode(v.0.data()),
//             sign: v.1.map(Into::into),
//         })
//     }
// }

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct HashWrapperDto<T: Serialize> {
    #[serde(flatten)]
    object: T,
    tx_hash: Hash,
}

impl<T: Serialize> HashWrapperDto<T> {
    pub fn into_hash_wrapper(data: T, hash: Hash) -> HashWrapperDto<T> {
        HashWrapperDto {
            object: data,
            tx_hash: hash,
        }
    }
}

#[cfg(feature = "internal_api")]
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct MemberEsiaTokenDto {
    token: String,
    oid: String,
}

#[cfg(feature = "internal_api")]
impl From<MemberEsiaToken> for MemberEsiaTokenDto {
    fn from(value: MemberEsiaToken) -> Self {
        Self {
            token: value.token().to_string(),
            oid: value.oid().to_string(),
        }
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
        //     "opening_time": "2020-12-10T02:00:53+00:00",
        //     "closing_time": "2020-12-31T05:00:53+00:00",
        // }
        LotInfo {
            name: "My Lot 1".to_string(),
            desc: "Explicit lot description".to_string(),
            price: Cost::from(5000000),
            opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
            closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
        }
    }

    pub fn new_object_ownership_info() -> ObjectOwnershipInfo {
        // {
        //     "object": {"class":1,"reg_number":"123451"},
        //     "contract_term": { "specification": "forever" },
        //     "exclusive": false,
        //     "can_distribute": "unable",
        //     "location": [{"registry": 1, "code": 45379000,"desc":""}],
        //     "classifier": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}]
        // }
        ObjectOwnershipInfo {
            object: ObjectIdentity::from_str("trademark::123451")
                .unwrap()
                .into(),
            contract_term: Default::default(),
            exclusive: false,
            can_distribute: Distribution::Unable,
            location: vec![Location::from_str("oktmo::45379000").unwrap().into()],
            classifiers: vec![
                Classifier::from_str("mktu::8").unwrap().into(),
                Classifier::from_str("mktu::13").unwrap().into(),
            ],
        }
    }

    pub fn new_conditions_info() -> ConditionsInfo {
        // {
        //     "contract_type": "license",
        //     "objects": {
        //         "object": {"class":1,"reg_number":"123451"},
        //         "contract_term": { "specification": "forever" },
        //         "exclusive": false,
        //         "can_distribute": "unable",
        //         "location": {"registry": 1, "code": 45379000,"desc":""},
        //         "classifier": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}],
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
        //     "rightholder": {"class":0,"number":"5068681643685"},
        //     "contract_type": "license",
        //     "exclusive": true,
        //     "can_distribute": "able",
        //     "location": [ {"registry": 1, "code": 45379000, "desc":""} ],
        //     "classifiers": [ {"registry": 1,"value": "8"} ],
        //     "starting_time": "2020-06-01T00:00:00Z",
        //     "expiration_time": "2021-06-01T00:00:00Z"
        // }
        StructuredOwnershipInfo {
            rightholder: MemberIdentity::from_str("ogrn::5068681643685")
                .unwrap()
                .into(),
            contract_type: ContractType::License,
            exclusive: true,
            can_distribute: Distribution::Able,
            location: vec![Location::from_str("oktmo::45379000").unwrap().into()],
            classifiers: vec![Classifier::from_str("mktu::8").unwrap().into()],
            starting_time: DateTime::<Utc>::from_str("2020-06-01T00:00:00Z").unwrap(),
            expiration_time: Some(DateTime::<Utc>::from_str("2021-06-01T00:00:00Z").unwrap()),
        }
    }

    #[test]
    fn de_ownership_info_structured() {
        let object_json = r#"{
            "representation": "structured",
            "rightholder": {"class":0,"number":"5068681643685"},
            "contract_type": "license",
            "exclusive": true,
            "can_distribute": "able",
            "location": [ {"registry": 1, "code": 45379000,"desc":""} ],
            "classifiers": [ {"registry": 1,"value": "8"} ],
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
            "rightholder": {"class":0,"number":"5068681643685"}
        }"#;
        let expected = OwnershipInfo::Unstructured(UnstructuredOwnershipInfo {
            rightholder: Some(
                MemberIdentity::from_str("ogrn::5068681643685")
                    .unwrap()
                    .into(),
            ),
            data: None,
            exclusive: None,
        });
        let deserialized: OwnershipInfo = serde_json::from_str(object_json).unwrap();
        assert_eq!(deserialized, expected)
    }

    // #[test]
    // fn de_object_info() {
    //     let object_json = vec![
    //         r#""trademark::2020630178""#,
    //         r#""wellknown_trademark::2020630178""#,
    //         r#""appellation_of_origin::2020630178""#,
    //         r#""appellation_of_origin_rights::2020630178/2354""#,
    //         r#""pharmaceutical::2020630178""#,
    //         r#""invention::2020630178""#,
    //         r#""utility_model::2020630178""#,
    //         r#""industrial_model::2020630178""#,
    //         r#""tims::2020630178""#,
    //         r#""program::2020630178""#,
    //         r#""database::2020630178""#,
    //     ];
    //     let parsed = object_json
    //         .iter()
    //         .map(|value| serde_json::from_str(value).expect("Unable to parse ObjectInfo"))
    //         .collect::<Vec<ObjectInfo>>();
    //     for x in object_json.into_iter().zip(parsed) {
    //         let right = format!("\"{}\"", x.1);
    //         assert_eq!(x.0, &right);
    //     }
    // }

    #[test]
    fn ser_object_info() {
        use std::str::FromStr;

        let data: ObjectIdentityDto = ObjectIdentity::from_str("trademark::123").unwrap().into();
        let result = serde_json::to_string(&data).expect("Unable to serialize ObjectInfo");
        assert_eq!(result, r#"{"class":1,"reg_number":"123"}"#);
    }

    #[test]
    fn de_member_info() {
        use std::str::FromStr;

        let member_ogrn_json = r#"{"class":0,"number":"1053600591197"}"#;
        let parced_ogrn: MemberInfo =
            serde_json::from_str(member_ogrn_json).expect("Unable to parse MemberInfo");
        let expected_ogrn = MemberIdentity::from_str("ogrn::1053600591197")
            .unwrap()
            .into();
        assert_eq!(parced_ogrn, expected_ogrn);

        let member_snils_json = r#"{"class":2,"number":"02583651862"}"#;
        let parced_snils: MemberInfo =
            serde_json::from_str(member_snils_json).expect("Unable to parse MemberInfo");
        let expected_snils = MemberIdentity::from_str("snils::02583651862")
            .unwrap()
            .into();
        assert_eq!(parced_snils, expected_snils);
    }

    #[test]
    fn ser_member_info() {
        use std::str::FromStr;

        let ogrn_data = MemberIdentity::from_str("ogrn::1053600591197")
            .unwrap()
            .into();
        let ogrn_result = serde_json::to_string::<MemberInfo>(&ogrn_data)
            .expect("Unable to serialize MemberInfo");
        assert_eq!(ogrn_result, r#"{"class":0,"number":"1053600591197"}"#);

        let snils_data = MemberIdentity::from_str("snils::02583651862")
            .unwrap()
            .into();
        let snils_result = serde_json::to_string::<MemberInfo>(&snils_data)
            .expect("Unable to serialize MemberInfo");
        assert_eq!(snils_result, r#"{"class":2,"number":"02583651862"}"#);
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
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": { "specification": "forever" },
            "exclusive": false,
            "can_distribute": "unable",
            "location": [{"registry":1,"code":45379000}],
            "classifiers": [{"registry":1,"value":"8","desc":"1234"},{"registry": 1,"value":"13"}]
        }"#;

        let true_val = ObjectOwnershipInfo {
            object: ObjectIdentity::from_str("trademark::123451")
                .unwrap()
                .into(),
            contract_term: Default::default(),
            exclusive: false,
            can_distribute: Distribution::Unable,
            location: vec![Location::from_str("oktmo::45379000").unwrap().into()],
            classifiers: vec![
                Classifier::from_str("mktu::8::1234").unwrap().into(),
                Classifier::from_str("mktu::13").unwrap().into(),
            ],
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_object_ownership_info_with_nulls() {
        let json = r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": null
        }"#;

        let true_val = ObjectOwnershipInfo {
            object: ObjectIdentity::from_str("trademark::123451")
                .unwrap()
                .into(),
            contract_term: Default::default(),
            exclusive: false,
            can_distribute: Distribution::Unable,
            location: vec![Location::default().into()],
            classifiers: vec![Classifier::default().into()],
        };
        let val: ObjectOwnershipInfo = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    #[test]
    fn de_object_ownership_info_bad_value() {
        let json = vec![
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": "bad_value",
            "classifiers": null
        }"#,
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": "bad_value"
        }"#,
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": 1,
            "classifiers": null
        }"#,
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": 1
        }"#,
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": null,
            "classifiers": {"bad_parametr": "bad value"}
        }"#,
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": {"bad_parametr": "bad value"},
            "classifiers": null
        }"#,
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
            "contract_term": null,
            "exclusive": false,
            "can_distribute": "unable",
            "location": [""],
            "classifiers": null
        }"#,
            r#"
        {
            "object": {"class":1,"reg_number":"123451"},
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
    fn se_conditions_info() {
        let json = r#"{"contract_type":"license","objects":[{"object":{"class":1,"reg_number":"123451"},"contract_term":{"specification":"forever"},"exclusive":false,"can_distribute":"unable","location":[{"registry":1,"code":45379000,"desc":""}],"classifiers":[{"registry":1,"value":"8","desc":""},{"registry":1,"value":"13","desc":""}]}],"payment_conditions":"Condition desc text","payment_comment":"test text","termination_conditions":["Term cond 1","Term cond 2"],"contract_extras":["Extra comment"]}"#;

        let true_val = ConditionsInfo {
            contract_type: ContractType::License,
            objects: vec![new_object_ownership_info()],
            payment_conditions: "Condition desc text".to_string(),
            payment_comment: "test text".to_string(),
            termination_conditions: vec!["Term cond 1".to_string(), "Term cond 2".to_string()],
            contract_extras: vec!["Extra comment".to_string()],
        };
        let val = serde_json::to_string(&true_val).unwrap();
        assert_eq!(json, val);
    }

    #[test]
    fn de_conditions_info() {
        let json = r#"
        {
            "contract_type": "license",
            "objects": [{
                "object": {"class":1,"reg_number":"123451"},
                "contract_term": { "specification": "forever" },
                "exclusive": false,
                "can_distribute": "unable",
                "location": [{"registry": 1, "code": 45379000,"desc":""}],
                "classifiers": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}]
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
                "object": {"class":1,"reg_number":"123451"},
                "contract_term": { "specification": "forever" },
                "exclusive": false,
                "can_distribute": "unable",
                "location": [{"registry": 1, "code": 45379000,"desc":""}],
                "classifiers": [{"registry": 1,"value": "8"}, {"registry": 1,"value": "13"}]
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
                "opening_time": "2020-12-10T02:00:53+00:00",
                "closing_time": "2020-12-31T05:00:53+00:00"
            }"#;
        let true_val = LotInfo {
            name: "My Lot 1".to_string(),
            desc: "Explicit lot description".to_string(),
            price: Cost::from(50000),
            opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
            closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
        };
        let val = serde_json::from_str(json).unwrap();
        assert_eq!(true_val, val);
    }

    // #[test]
    // fn se_lot_info() {
    //     let true_json = r#"{"name":"My Lot 1","desc":"Explicit lot description","price":50000,"sale_type":"auction","opening_time":"2020-12-10T02:00:53Z","closing_time":"2020-12-31T05:00:53Z","status":"undefined"}"#;
    //     let val = LotInfo {
    //         name: "My Lot 1".to_string(),
    //         desc: "Explicit lot description".to_string(),
    //         price: Cost::from(50000),
    //         sale_type: SaleType::Auction,
    //         opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
    //         closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
    //         status: Default::default(),
    //     };
    //     let val = serde_json::to_string(&val).unwrap();
    //     assert_eq!(val, true_json);
    // }

    // #[test]
    // fn de_lot_info_with_objects() {
    //     let json = r#"{
    //             "name": "My Lot 1",
    //             "desc": "Explicit lot description",
    //             "price": 50000,
    //             "sale_type": "auction",
    //             "opening_time": "2020-12-10T02:00:53+00:00",
    //             "closing_time": "2020-12-31T05:00:53+00:00",
    //             "status": "undefined",
    //             "objects": ["trademark::123"]
    //         }"#;
    //     let true_val = LotInfoWithObjects {
    //         lot: LotInfo {
    //             name: "My Lot 1".to_string(),
    //             desc: "Explicit lot description".to_string(),
    //             price: Cost::from(50000),
    //             sale_type: SaleType::Auction,
    //             opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
    //             closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
    //             status: Default::default(),
    //         },
    //         objects: vec![ObjectInfo(
    //             ObjectIdentity::from_str("trademark::123").unwrap(),
    //         )],
    //     };
    //     let val = serde_json::from_str(json).unwrap();
    //     assert_eq!(true_val, val);
    // }

    #[test]
    fn se_lot_info_with_objects() {
        let true_json = r#"{"name":"My Lot 1","desc":"Explicit lot description","seller":{"class":0,"number":"1053600591197"},"price":50000,"sale_type":"auction","opening_time":"2020-12-10T02:00:53Z","closing_time":"2020-12-31T05:00:53Z","status":"new","is_undefined":false,"conditions":{"contract_type":"license","objects":[{"object":{"class":1,"reg_number":"123451"},"contract_term":{"specification":"forever"},"exclusive":false,"can_distribute":"unable","location":[{"registry":1,"code":45379000,"desc":""}],"classifiers":[{"registry":1,"value":"8","desc":""},{"registry":1,"value":"13","desc":""}]}],"payment_conditions":"Condition desc text","payment_comment":"test text","termination_conditions":["Term cond 1","Term cond 2"],"contract_extras":["Extra comment"]},"calculations":[],"reference_number":null}"#;
        let val = LotInfoWithObjects {
            name: "My Lot 1".to_string(),
            desc: "Explicit lot description".to_string(),
            seller: MemberIdentity::from_str("ogrn::1053600591197")
                .unwrap()
                .into(),
            price: Cost::from(50000),
            sale_type: SaleType::Auction,
            opening_time: DateTime::<Utc>::from_str("2020-12-10T02:00:53+00:00").unwrap(),
            closing_time: DateTime::<Utc>::from_str("2020-12-31T05:00:53+00:00").unwrap(),
            status: LotStatus::New,
            is_undefined: false,
            conditions: ConditionsInfo {
                contract_type: ContractType::License,
                objects: vec![new_object_ownership_info()],
                payment_conditions: "Condition desc text".to_string(),
                payment_comment: "test text".to_string(),
                termination_conditions: vec!["Term cond 1".to_string(), "Term cond 2".to_string()],
                contract_extras: vec!["Extra comment".to_string()],
            },
            calculations: vec![],
            reference_number: None,
        };
        let val = serde_json::to_string(&val).unwrap();
        assert_eq!(val, true_json);
    }

    #[test]
    fn de_calculation_info() {
        let json = r#"{
                "id": "1234",
                "data": "asdf"
            }"#;
        let _true_val = CalculationInfo {
            id: "1234".to_string(),
            data: "asdf".to_string(),
            timestamp: Utc::now(),
        };
        let _val: CalculationInfo = serde_json::from_str(json).unwrap();
    }
}
