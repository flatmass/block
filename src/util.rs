use crate::error::{Error, Result};
use std::collections::HashMap;
use std::str::FromStr;

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
