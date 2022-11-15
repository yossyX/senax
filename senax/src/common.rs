use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::convert::TryInto;

use crate::schema::BAD_KEYWORDS;

pub fn hash(v: &str) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(v);
    let result = hasher.finalize();
    let (int_bytes, _rest) = result.split_at(std::mem::size_of::<u64>());
    u64::from_ne_bytes(int_bytes.try_into().unwrap())
}

pub fn check_name(name: &str) -> &str {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\p{XID_Start}\p{XID_Continue}*$").unwrap());
    if !RE.is_match(name) || BAD_KEYWORDS.iter().any(|&x| x == name) {
        panic!("{} is an incorrect name.", name)
    }
    name
}

pub fn check_ascii_name(name: &str) -> &str {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z][0-9A-Z_a-z]*$").unwrap());
    if !RE.is_match(name) || BAD_KEYWORDS.iter().any(|&x| x == name) {
        panic!("{} is an incorrect name.", name)
    }
    name
}

pub fn is_ascii_name(name: &str) -> bool {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Za-z][0-9A-Z_a-z]*$").unwrap());
    RE.is_match(name)
}
macro_rules! if_then_else {
    ( $if:expr, $then:expr, $else:expr ) => {
        if $if {
            $then
        } else {
            $else
        }
    };
}
pub(crate) use if_then_else;
