use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use num_enum::TryFromPrimitive;

use blockp_core::storage::Snapshot;

use crate::error::Error;
use crate::schema::Schema;
use crate::util::contains_diplicates;

use super::classifier::Classifier;
use super::location::Location;
use super::member::MemberIdentity;
use super::object::ObjectIdentity;
use super::time::Term;

#[repr(u8)]
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, TryFromPrimitive)]
#[serde(rename_all = "snake_case")]
pub enum ContractType {
    Undefined = 0,
    License = 1,
    Sublicense = 2,
    ConcessionAgreement = 4,
    SubconcessionAgreement = 8,
    Expropriation = 16,
}

impl FromStr for ContractType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_plain::from_str(s).map_err(|_| Error::bad_contract_type_format(s))
    }
}

impl Display for ContractType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_plain::to_string(self).expect("serde_plain::to_string failed on ContractType")
        )
    }
}

encoding_struct! {
    struct ExtraCondition {
        info: &str
    }
}

impl From<&str> for ExtraCondition {
    fn from(src: &str) -> Self {
        ExtraCondition::new(src)
    }
}

encoding_struct! {
    struct TerminationCondition {
        info: &str
    }
}

impl From<&str> for TerminationCondition {
    fn from(src: &str) -> Self {
        TerminationCondition::new(src)
    }
}

encoding_struct! {
    struct Conditions {
        /// ContractType
        contract_type: u8,
        objects: Vec<ObjectOwnership>,
        payment_conditions: &str,
        payment_comment: &str,
        termination_conditions: Vec<TerminationCondition>,
        contract_extras: Vec<ExtraCondition>,
    }
}

impl Conditions {
    pub fn is_concession_agreement(&self) -> bool {
        self.contract_type() == ContractType::ConcessionAgreement as u8
            || self.contract_type() == ContractType::SubconcessionAgreement as u8
    }

    pub fn is_expropriation(&self) -> bool {
        self.contract_type() == ContractType::Expropriation as u8
    }

    fn contains_trademark(&self) -> bool {
        self.objects().iter().any(|o| o.object().is_trademark())
    }

    pub fn check(&self) -> Vec<Check> {
        let mut results = Vec::new();
        results.push(self.check_locations());
        results.push(self.check_duplicate_objects());

        if !self.contains_trademark() {
            results.push(self.check_objects_sellable());
        }

        if self.is_concession_agreement() {
            results.push(self.check_contains_tm());
        }

        results
    }

    pub fn check_seller(&self, seller: &MemberIdentity) -> Check {
        if self.is_concession_agreement() && seller.is_person() {
            CheckKey::CanSell.err()
        } else {
            CheckKey::CanSell.ok()
        }
    }

    pub fn check_buyer(&self, buyer: &MemberIdentity) -> Check {
        if self.is_concession_agreement() && buyer.is_person() {
            CheckKey::CanBuy.err()
        } else if self.is_expropriation()
            && self.objects().iter().any(|o| o.object().is_trademark())
            && buyer.is_person()
        {
            CheckKey::CanBuy.err()
        } else {
            CheckKey::CanBuy.ok()
        }
    }

    pub fn check_rights<T>(
        &self,
        schema: &Schema<T>,
        seller: &MemberIdentity,
    ) -> Result<Vec<Check>, Error>
    where
        T: AsRef<dyn Snapshot>,
    {
        let mut results = Vec::new();

        let mut term_check = CheckKey::DurationValid.new_check_chain();
        let mut struct_check = CheckKey::NoUnstructuredData.new_check_chain();

        for obj_ownership in self.objects() {
            let object = obj_ownership.object();
            let obj_id = &object.id();
            if !schema.objects().contains(obj_id) {
                Error::no_object(&object).ok()?
            }

            // Some ownership information is unstructured
            if !schema.ownership_unstructured(obj_id).is_empty() {
                struct_check.and(0);
                term_check.and(0);
            }
            // All ownership information is structured
            else if let Some(rights) = schema.rights(&seller.id(), obj_id) {
                struct_check.and(1);
                term_check.and(rights.check_term(&object, obj_ownership.contract_term())?);
            }
            // No ownership information found
            else {
                Error::no_permissions().ok()?
            }
        }

        results.push(term_check.finalize());
        results.push(struct_check.finalize());

        Ok(results)
    }

    fn check_locations(&self) -> Check {
        if self.objects().iter().all(|o| o.all_locations_oktmo()) {
            CheckKey::LocationValid.ok()
        } else {
            CheckKey::LocationValid.unknown()
        }
    }

    fn check_duplicate_objects(&self) -> Check {
        let objects = self
            .objects()
            .iter()
            .map(|o| o.object())
            .collect::<Vec<ObjectIdentity>>();

        if contains_diplicates(objects) {
            CheckKey::ObjectDuplicates.err()
        } else {
            CheckKey::ObjectDuplicates.ok()
        }
    }

    fn check_objects_sellable(&self) -> Check {
        if self.objects().iter().all(|o| o.object().is_sellable()) {
            CheckKey::ObjectsSellable.ok()
        } else {
            CheckKey::ObjectsSellable.err()
        }
    }

    fn check_contains_tm(&self) -> Check {
        if self.contains_trademark() {
            CheckKey::ContainsTrademark.ok()
        } else {
            CheckKey::ContainsTrademark.err()
        }
    }
}

impl Default for Conditions {
    fn default() -> Self {
        Self::new(
            ContractType::Undefined as u8,
            vec![],
            "",
            "",
            vec![],
            vec![],
        )
    }
}

encoding_struct! {
    struct ObjectOwnership {
        object: ObjectIdentity,
        contract_term: Term,
        exclusive: bool,
        can_distribute: u8,
        location: Vec<Location>,
        classifiers: Vec<Classifier>,
    }
}

impl ObjectOwnership {
    fn all_locations_oktmo(&self) -> bool {
        for location in self.location() {
            if !location.is_oktmo() {
                return false;
            }
        }
        true
    }
}

encoding_struct! {
    #[derive(Eq)]
    struct CheckResult {
        result: i8,
        desc: &str,
    }
}

impl CheckResult {
    pub fn is_error(&self) -> bool {
        self.result() < 0
    }
}

#[repr(u16)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize, TryFromPrimitive, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CheckKey {
    // Internal checks
    /// Document list was generated automatically based on contract conditions
    #[serde(skip_deserializing)]
    DocumentsMatchCondition = 0,
    #[serde(skip_deserializing)]
    CanSell,
    #[serde(skip_deserializing)]
    CanBuy,
    #[serde(skip_deserializing)]
    LocationValid,
    #[serde(skip_deserializing)]
    ObjectDuplicates,
    #[serde(skip_deserializing)]
    ObjectsSellable,
    #[serde(skip_deserializing)]
    ContainsTrademark,
    #[serde(skip_deserializing)]
    NoUnstructuredData,

    // External checks
    TaxPaymentInfoAdded = 32768, // 9
    Blacklist,                   // 8
    SellerDataValid,             // 2.1
    DurationValid,               // 7
    UsecasesMatch,               // 4.6
    RegisteredChanges,           // 5.2
    PublicExpropriationOffer,    // 5.3
}

impl CheckKey {
    pub fn ok(self) -> Check {
        let desc = match self {
            CheckKey::DocumentsMatchCondition => "Комплект документов соответствует условиям контракта",
            CheckKey::CanSell => "Продавец соответствует требованиям ГК РФ п. 3 ст. 1027",
            CheckKey::CanBuy => "Покупатель соответствует требованиям ГК РФ п. 3 ст. 1027",
            CheckKey::LocationValid => "Территоиря указана согласно справочнику ФИАС",
            CheckKey::ObjectDuplicates => "ОИС проверяемого вида могут участвать в сделке",
            CheckKey::ObjectsSellable => "ОИС проверяемого вида могут участвать в сделке",
            CheckKey::ContainsTrademark => "ТЗ присутствует в сделке",
            CheckKey::NoUnstructuredData => "Вся информация о владении ОИС структурирована, возможна автоматическая обработка",
            CheckKey::TaxPaymentInfoAdded => "Полученные данные подтверждают уплату пошлины в необходимом размере и требуемые сроки",
            CheckKey::DurationValid => "Текущая дата меньше установленной даты окончания срока действия исключительного права",
            CheckKey::Blacklist => "Действующие записи отсутствуют в списке",
            _ => "",
        };
        self.result(1, desc)
    }

    pub fn err(self) -> Check {
        let desc = match self {
            CheckKey::CanSell => "Продавец не соответствует требованиям ГК РФ п. 3 ст. 1027",
            CheckKey::CanBuy => "Покупатель не соответствует требованиям ГК РФ п. 3 ст. 1027",
            CheckKey::ObjectDuplicates => "ОИС проверяемого вида не могут участвать в сделке",
            CheckKey::ObjectsSellable => "ОИС проверяемого вида не могут участвать в сделке",
            CheckKey::ContainsTrademark => "ТЗ не присутствует в сделке",
            CheckKey::TaxPaymentInfoAdded => "Полученные данные свидетельствуют об отсутствии уплаты пошлины в необходимом размере и требуемые сроки",
            CheckKey::DurationValid => "Текущая дата больше установленной даты окончания срока действия исключительного права",
            _ => "",
        };
        self.result(-1, desc)
    }

    pub fn unknown(self) -> Check {
        let desc = match self {
            CheckKey::DocumentsMatchCondition => "Невозможно проверить соответствие документов условиям контракта",
            CheckKey::NoUnstructuredData => "Присутствует неструктурированая информация о владении ОИС, автоматическая обработка невозможна",
            CheckKey::DurationValid => "",
            CheckKey::LocationValid => "Применен свободный ввод территории",
            CheckKey::Blacklist => "Действующие записи присутствуют в списке",
            _ => "",
        };
        self.result(0, desc)
    }

    pub fn code(self, code: i8) -> Check {
        if code < 0 {
            self.err()
        } else if code > 0 {
            self.ok()
        } else {
            self.unknown()
        }
    }

    pub fn new_check_chain(self) -> CheckBuilder {
        CheckBuilder { key: self, code: 1 }
    }

    fn result(self, success: i8, desc: &str) -> Check {
        Check::new(self as u16, CheckResult::new(success, desc))
    }
}

pub struct CheckBuilder {
    key: CheckKey,
    code: i8,
}

impl CheckBuilder {
    // Keep worst result
    pub fn and(&mut self, code: i8) -> &CheckBuilder {
        self.code = std::cmp::min(code, self.code);
        self
    }

    pub fn finalize(self) -> Check {
        self.key.code(self.code)
    }
}

encoding_struct! {
    struct Check {
        key: u16,
        result: CheckResult,
    }
}

impl From<(u16, CheckResult)> for Check {
    fn from((key, result): (u16, CheckResult)) -> Self {
        Self::new(key, result)
    }
}
