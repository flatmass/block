use crate::data::member::{MemberIdentity, MemberType};
use crate::error::{Error as FipsError, Result};
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::time::Duration;

fn send_request<T: for<'de> serde::Deserialize<'de>>(
    duration: Duration,
    url: impl IntoUrl,
    bearer: &str,
) -> Result<T> {
    let resp = reqwest::blocking::Client::builder()
        .timeout(duration)
        .build()?
        .get(url)
        .bearer_auth(bearer)
        .send()?;

    let status_code = resp.status();

    if !status_code.is_success() {
        let body: serde_json::Value = resp.json()?;
        return FipsError::while_requesting(&status_code, body).ok();
    }
    Ok(resp.json()?)
}

#[cfg(feature = "esia_test")]
const ESIA_URL: &str = "https://esia-portal1.test.gosuslugi.ru";
#[cfg(not(feature = "esia_test"))]
const ESIA_URL: &str = "https://esia.gosuslugi.ru";

pub struct EsiaAuth;

impl EsiaAuth {
    pub fn validate(member: &MemberIdentity, bearer_token: &str, oid: &str) -> Result<bool> {
        #[cfg(feature = "no_esia_reqwest")]
        return Ok(true);

        let member_type: MemberType = member.class().try_into().unwrap();
        match member_type {
            MemberType::Ogrn => Self::validate_ogrn(member.number(), bearer_token, oid),
            MemberType::Ogrnip => Self::validate_ogrn(member.number(), bearer_token, oid),
            MemberType::Snils => Self::validate_snils(member.number(), bearer_token, oid),
        }
    }

    fn validate_snils(member_number: &str, bearer_token: &str, oid: &str) -> Result<bool> {
        let url = format!("{}/rs/prns/{}", ESIA_URL, oid);

        let esia_response: EsiaOidProfile =
            send_request(Duration::from_secs(1), url, bearer_token)?;

        Ok(esia_response.compare_with_member_number(member_number))
    }

    fn validate_ogrn(member_number: &str, bearer_token: &str, oid: &str) -> Result<bool> {
        let url = format!("{}/rs/prns/{}/roles", ESIA_URL, oid);

        let esia_response: EsiaOidRoles = send_request(Duration::from_secs(1), url, bearer_token)?;

        let is_success = esia_response
            .elements
            .into_iter()
            .any(|ref element| element.ogrn == member_number);
        Ok(is_success)
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct EsiaOidProfile {
    snils: String,
}

impl EsiaOidProfile {
    pub fn compare_with_member_number(&self, member_number: &str) -> bool {
        self.snils
            .replace("-", "")
            .replace(" ", "")
            .eq(member_number)
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct EsiaOidRoles {
    pub elements: Vec<EsiaOidRolesElement>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
struct EsiaOidRolesElement {
    pub ogrn: String,
}
