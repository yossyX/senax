#![allow(unused_variables)]
use anyhow::Result;
use std::path::PathBuf;

use super::msec::MSec;

pub struct DiskCache;
impl DiskCache {
    pub fn start(
        path: PathBuf,
        index_size: u64,
        file_num: usize,
        file_size: u64,
        ttl: u64,
    ) -> Result<DiskCache, anyhow::Error> {
        Ok(DiskCache)
    }

    pub fn stop(&self) {}

    pub fn write(&self, hash: u128, type_id: u64, data: &[u8], time: MSec) {}

    pub async fn read(&self, hash: u128, type_id: u64, estimate: usize) -> Option<Vec<u8>> {
        None
    }

    pub fn invalidate_all_of(&self, type_id: u64) {}

    pub fn invalidate_all(&self) {}
}
