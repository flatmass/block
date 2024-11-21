use std::collections::HashMap;
use std::str::FromStr;

use chrono::Utc;

use crate::data::attachment::{Attachment, AttachmentMetadata, AttachmentType};
use crate::error::{Error, Result};

#[allow(unused)]
pub fn utf8_str_param<'a>(name: &'_ str, params: &'a HashMap<String, Vec<u8>>) -> Result<&'a str> {
    params
        .get(name)
        .ok_or(Error::no_param(name).into())
        .and_then(|v| {
            if v.len() == 0 {
                Err(Error::empty_param(name))
            } else {
                std::str::from_utf8(&v).map_err(Error::from)
            }
        })
}

pub fn get_from_map<T, E>(map: &HashMap<String, String>, name: &str) -> Result<T>
where
    E: Into<Error>,
    T: FromStr<Err = E>,
{
    map.get(name)
        .ok_or(Error::no_param(name))
        .and_then(|s| {
            if s.is_empty() {
                Err(Error::empty_param(name))
            } else {
                Ok(s)
            }
        })?
        .parse()
        .map_err(Into::into)
}

pub fn get_from_map_nullable<T, E>(
    map: &HashMap<String, impl AsRef<[u8]>>,
    name: &str,
) -> Result<Option<T>>
where
    E: Into<Error>,
    T: FromStr<Err = E>,
{
    let value = match map.get(name) {
        None => None,
        Some(v) => {
            if v.as_ref().is_empty() {
                Err(Error::empty_param(name))?
            } else {
                let str = std::str::from_utf8(v.as_ref())?;
                Some(str.parse::<T>().map_err(Into::into)?)
            }
        }
    };
    Ok(value)
}

#[cfg(feature = "internal_api")]
pub fn get_from_map_nullable_str<'a>(
    map: &'a HashMap<String, impl AsRef<[u8]>>,
    name: &str,
) -> Result<Option<&'a str>> {
    let value = map
        .get(name)
        .map(|v| std::str::from_utf8(v.as_ref()))
        .transpose()?;
    Ok(value)
}

#[cfg(feature = "internal_api")]
pub fn get_from_map_nullable_slice<'a>(
    map: &'a HashMap<String, impl AsRef<[u8]>>,
    name: &str,
) -> Option<&'a [u8]> {
    map.get(name).map(|v| v.as_ref())
}

#[cfg(feature = "internal_api")]
pub fn get_from_map_nullable_string(
    map: &HashMap<String, impl AsRef<[u8]>>,
    name: &str,
) -> Option<String> {
    map.get(name)
        .map(|v| String::from_utf8_lossy(v.as_ref()).into_owned())
}

pub fn get_from_multipart_map<T>(map: &HashMap<String, impl AsRef<[u8]>>, name: &str) -> Result<T>
where
    T: FromStr<Err = Error>,
{
    get_str_from_map(map, name)?
        .parse::<T>()
        .map_err(Into::into)
}

pub fn get_str_from_map<'a>(
    map: &'a HashMap<String, impl AsRef<[u8]>>,
    name: &str,
) -> Result<&'a str> {
    get_slice_from_map(map, name)
        .and_then(|bytes: &[u8]| std::str::from_utf8(bytes).map_err(Into::into))
        .and_then(|s| {
            if s.is_empty() {
                Err(Error::empty_param(name))
            } else {
                Ok(s)
            }
        })
}

#[cfg(feature = "internal_api")]
pub fn get_string_from_map(map: &HashMap<String, impl AsRef<[u8]>>, name: &str) -> Result<String> {
    get_slice_from_map(map, name)
        .map(|bytes: &[u8]| String::from_utf8_lossy(bytes))
        .and_then(|s| {
            if s.is_empty() {
                Err(Error::empty_param(name))
            } else {
                Ok(s.to_string())
            }
        })
}

pub fn get_attachment_from_map(map: &HashMap<String, impl AsRef<[u8]>>) -> Result<Attachment> {
    let name = get_str_from_map(map, "name")?;
    let description = get_from_map_nullable(map, "description")?;
    let file_type: AttachmentType = get_from_multipart_map(map, "file_type")?;
    let file = get_slice_from_map(map, "file")?;
    let sign = get_from_map_nullable(map, "sign")?;
    let meta = AttachmentMetadata::new(name, description, file_type as u8, Utc::now());
    Ok(Attachment::new(meta, file, sign))
}

#[cfg(feature = "internal_api")]
pub fn get_attachment_nullable_from_map(
    map: &HashMap<String, impl AsRef<[u8]>>,
) -> Result<Option<Attachment>> {
    let name = get_from_map_nullable_str(map, "name")?;
    let description = get_from_map_nullable_string(map, "description");
    let file_type: Option<AttachmentType> = get_from_map_nullable(map, "file_type")?;
    let file = get_from_map_nullable_slice(map, "file");
    let sign = get_from_map_nullable(map, "sign")?;
    match (name, file_type, file, sign) {
        (None, None, None, None) => Ok(None),
        (name, file_tpye, file, sign) => {
            let name = name.ok_or_else(|| Error::empty_param("name"))?;
            let file_type = file_tpye.ok_or_else(|| Error::empty_param("file_tpye"))?;
            let file = file.ok_or_else(|| Error::empty_param("file"))?;
            let meta = AttachmentMetadata::new(name, description, file_type as u8, Utc::now());
            Ok(Some(Attachment::new(meta, file, sign)))
        }
    }
}

pub fn get_slice_from_map<'a>(
    map: &'a HashMap<String, impl AsRef<[u8]>>,
    name: &str,
) -> Result<&'a [u8]> {
    map.get(name)
        .map(|v| v.as_ref())
        .ok_or_else(|| Error::no_param(name))
        .and_then(|s| {
            if s.is_empty() {
                Err(Error::empty_param(name))
            } else {
                Ok(s)
            }
        })
}

// remove duplicates from Vector.
// O(N^2) performance; O(N) memory
#[allow(unused)]
pub fn dedup_naive<T>(vec: Vec<T>) -> Vec<T>
where
    T: Eq + Clone,
{
    let len = vec.len();
    if len > 32 {
        // naive algorithm is not expected to be used with big numbers.
        panic!("Unsupported share size: expected < 32");
    }
    if len == 0 || len == 1 {
        return vec;
    }

    // count duplicates and mark them
    let mut dups = vec![false; vec.len()];
    let mut dups_amount = 0;
    for i in 0..vec.len() - 1 {
        for j in i + 1..vec.len() {
            if vec[i] == vec[j] {
                dups[i] = true;
                dups_amount = dups_amount + 1;
                break;
            }
        }
    }
    let mut res: Vec<T> = Vec::with_capacity(len - dups_amount);
    let mut res_idx = 0;

    // collect duplicates
    for i in 0..len {
        if dups[i] == false {
            res.push(vec[i].clone());
            res_idx = res_idx + 1;
        }
    }
    res
}

// find duplicates in Vector.
// O(N^2) performance; O(1) memory
pub fn contains_diplicates<T>(vec: Vec<T>) -> bool
where
    T: Eq,
{
    if vec.len() < 2 {
        return false;
    }

    // find duplicates
    for i in 0..vec.len() - 1 {
        for j in i + 1..vec.len() {
            if vec[i] == vec[j] {
                return true;
            }
        }
    }

    false
}
