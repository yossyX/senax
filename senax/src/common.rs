use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::{convert::TryInto, fs, path::Path};

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

pub fn fs_write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> Result<()> {
    fn inner(path: &Path, contents: &[u8]) -> Result<()> {
        if let Ok(buf) = fs::read(path) {
            if !buf.eq(contents) {
                fs::write(path, contents)?;
            }
        } else {
            fs::write(path, contents)?;
        }
        Ok(())
    }
    inner(path.as_ref(), contents.as_ref())
}
