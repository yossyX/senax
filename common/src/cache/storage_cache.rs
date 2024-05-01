use anyhow::{bail, Result};
use arc_swap::ArcSwapOption;
use bytes::{Buf, BufMut, BytesMut};
use futures::future;
use fxhash::FxHashMap;
use log::error;
use std::sync::Arc;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    thread,
};
use tokio::sync::{
    mpsc::{self, Sender, UnboundedSender},
    oneshot,
};
use tokio_uring::fs::{File, OpenOptions};

use super::msec::MSec;

const ALIGNMENT_SHIFT: usize = 3;
const ALIGNMENT: u64 = 1 << ALIGNMENT_SHIFT;
const SET_ASSOCIATIVE: u64 = 16;
const HASH_SHIFT: usize = 40;
const POS_MASK: u64 = (1 << HASH_SHIFT) - 1;
const HEADER_SIZE: usize = 40;
const MAX_FILE_SIZE: u64 = 4398046511104;

pub struct StorageCache(Vec<StorageCacheInner>);
impl StorageCache {
    pub fn start(
        path: PathBuf,
        index_size: u64,
        file_num: usize,
        file_size: u64,
        ttl: u64,
    ) -> Result<StorageCache, anyhow::Error> {
        let mut vec = Vec::with_capacity(file_num);
        for i in 0..file_num {
            vec.push(_start(
                path.with_extension(i.to_string()),
                index_size,
                file_size,
                ttl,
            )?);
        }
        Ok(StorageCache(vec))
    }

    pub fn stop(&self) {
        for i in 0..self.0.len() {
            let _ = self.0[i].controller.send(Ctrl::Terminate);
        }
    }

    pub fn write(&self, hash: u128, type_id: u64, data: &[u8], time: MSec) {
        self.0[hash as usize % self.0.len()].write(hash, type_id, data, time);
    }

    pub async fn read(&self, hash: u128, type_id: u64, estimate: usize) -> Option<Vec<u8>> {
        self.0[hash as usize % self.0.len()]
            .read(hash, type_id, estimate)
            .await
    }

    pub fn invalidate_all_of(&self, type_id: u64) {
        for i in 0..self.0.len() {
            let _ = self.0[i].controller.send(Ctrl::InvalidateAllOf(type_id));
        }
    }

    pub fn invalidate_all(&self) {
        for i in 0..self.0.len() {
            self.0[i].invalidate_all();
        }
    }
}

struct StorageCacheInner {
    writer: UnboundedSender<WriteData>,
    reader: Sender<ReadData>,
    index: Arc<ArcSwapOption<Vec<AtomicU64>>>,
    controller: UnboundedSender<Ctrl>,
    file_size: u64,
    file_pos: Arc<AtomicU64>,
    outdated_pos: Arc<AtomicU64>,
    ttl: u64,
}

impl StorageCacheInner {
    fn write(&self, hash: u128, type_id: u64, data: &[u8], time: MSec) {
        if data.len() >= u32::MAX as usize - HEADER_SIZE {
            return;
        }
        if time.less_than_ttl(MSec::now(), self.ttl) {
            return;
        }
        let mut buf = BytesMut::with_capacity(HEADER_SIZE + data.len());
        buf.put_u128_le(hash);
        buf.put_u64_le(type_id);
        buf.put_u64_le(time.inner());
        buf.put_u32_le(data.len() as u32);
        buf.put_u32_le(checksum(data));
        buf.put(data);
        buf.resize(
            (buf.len() + ALIGNMENT as usize - 1) & !(ALIGNMENT as usize - 1),
            0,
        );
        let _ = self.writer.send(WriteData {
            hash,
            type_id,
            time,
            buf,
        });
    }
    async fn read(&self, hash: u128, type_id: u64, estimate: usize) -> Option<Vec<u8>> {
        get_index(
            &self.index,
            hash,
            &self.file_pos,
            self.file_size,
            &self.outdated_pos,
        )?;
        let (sender, receiver) = oneshot::channel::<Option<Vec<u8>>>();
        if self
            .reader
            .send(ReadData {
                hash,
                type_id,
                estimate,
                sender,
            })
            .await
            .is_ok()
        {
            return receiver.await.ok().flatten();
        }
        None
    }

    pub fn invalidate_all(&self) {
        let pos = self.file_pos.load(Ordering::Relaxed);
        self.outdated_pos.store(pos, Ordering::Relaxed);
    }
}

struct WriteData {
    hash: u128,
    type_id: u64,
    time: MSec,
    buf: BytesMut,
}

struct ReadData {
    hash: u128,
    type_id: u64,
    estimate: usize,
    sender: oneshot::Sender<Option<Vec<u8>>>,
}

enum Ctrl {
    InvalidateAllOf(u64),
    Terminate,
}

struct CacheFile(PathBuf);

impl Drop for CacheFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn _start(
    path: PathBuf,
    index_size: u64,
    file_size: u64,
    ttl: u64,
) -> Result<StorageCacheInner, anyhow::Error> {
    let index_size = (index_size / std::mem::size_of::<AtomicU64>() as u64) as usize;
    let index_size =
        1usize << (std::mem::size_of::<usize>() as u32 * 8 - index_size.leading_zeros() - 1);
    let file_size = std::cmp::min(file_size, MAX_FILE_SIZE);
    let (writer, mut writer_rx) = mpsc::unbounded_channel::<WriteData>();
    let (reader, mut reader_rx) = mpsc::channel::<ReadData>(256);
    let (controller, mut controller_rx) = mpsc::unbounded_channel::<Ctrl>();
    let index: Arc<ArcSwapOption<Vec<AtomicU64>>> = Arc::new(ArcSwapOption::const_empty());
    let _index = Arc::clone(&index);
    let file_pos = Arc::new(AtomicU64::new(0));
    let _file_pos = Arc::clone(&file_pos);
    let outdated_pos = Arc::new(AtomicU64::new(u64::MAX));
    let _outdated_pos = Arc::clone(&outdated_pos);

    thread::Builder::new()
        .name("disk cache".to_string())
        .spawn(move || {
            tokio_uring::start(async move {
                let index = &_index;
                let file_pos = &_file_pos;
                let outdated_pos = &_outdated_pos;
                let mut outdated_map: FxHashMap<u64, MSec> = FxHashMap::default();
                let file = match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(&path)
                    .await {
                        Ok(file) => file,
                        Err(err) => {
                            error!("{}", err);
                            return;
                        }
                    };
                let _cache_file = CacheFile(path);
                loop {
                    tokio::select! {
                        Some(ctrl) = controller_rx.recv() => {
                            match ctrl {
                                Ctrl::InvalidateAllOf(type_id) => {
                                    outdated_map.insert(type_id, MSec::now().add(1));
                                },
                                Ctrl::Terminate => {
                                    break;
                                },
                            }
                        },
                        Some(recv) = writer_rx.recv() => {
                            init_index(index, index_size);
                            let mut writer_handles = Vec::new();
                            writer_handles.push(handle_write(&file, index, recv, file_pos, file_size, &outdated_map, ttl));
                            while let Ok(recv) = writer_rx.try_recv() {
                                writer_handles.push(handle_write(&file, index, recv, file_pos, file_size, &outdated_map, ttl));
                            }
                            let mut reader_handles = Vec::new();
                            while let Ok(recv) = reader_rx.try_recv() {
                                reader_handles.push(handle_read(&file, index, recv, file_pos, file_size, outdated_pos, &outdated_map, ttl));
                            }
                            tokio::join!(future::join_all(reader_handles), future::join_all(writer_handles));
                        },
                        Some(recv) = reader_rx.recv() => {
                            if index.load().is_none() {
                                let _ = recv.sender.send(None);
                                continue;
                            }
                            let mut reader_handles = Vec::new();
                            reader_handles.push(handle_read(&file, index, recv, file_pos, file_size, outdated_pos, &outdated_map, ttl));
                            while let Ok(recv) = reader_rx.try_recv() {
                                reader_handles.push(handle_read(&file, index, recv, file_pos, file_size, outdated_pos, &outdated_map, ttl));
                            }
                            let mut writer_handles = Vec::new();
                            while let Ok(recv) = writer_rx.try_recv() {
                                init_index(index, index_size);
                                writer_handles.push(handle_write(&file, index, recv, file_pos, file_size, &outdated_map, ttl));
                            }
                            future::join(future::join_all(reader_handles), future::join_all(writer_handles)).await;
                        },
                        else => break,
                    }
                }
                index.swap(None);
            })
        })?;
    Ok(StorageCacheInner {
        writer,
        reader,
        index,
        controller,
        file_size,
        file_pos,
        outdated_pos,
        ttl,
    })
}

async fn handle_write(
    file: &File,
    index: &Arc<ArcSwapOption<Vec<AtomicU64>>>,
    recv: WriteData,
    file_pos: &AtomicU64,
    file_size: u64,
    outdated_map: &FxHashMap<u64, MSec>,
    ttl: u64,
) {
    let size = recv.buf.len() as u64;
    if size > file_size / 2 {
        return;
    }
    let mut time = MSec::now().sub(ttl);
    if let Some(outdated) = outdated_map.get(&recv.type_id) {
        if time.less_than(*outdated) {
            time = *outdated;
        }
    }
    if recv.time.less_than(time) {
        return;
    }
    let mut pos = file_pos.load(Ordering::Relaxed);
    if (pos % file_size) + size > file_size {
        pos = pos.wrapping_add(file_size).saturating_sub(pos % file_size);
    }
    file_pos.store(
        pos.wrapping_add((size + ALIGNMENT - 1) & !(ALIGNMENT - 1)),
        Ordering::Relaxed,
    );
    match write(file, pos % file_size, recv.buf).await {
        Ok(_) => {
            set_index(index, recv.hash, pos);
        }
        Err(err) => {
            error!("{}", err);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_read(
    file: &File,
    index: &Arc<ArcSwapOption<Vec<AtomicU64>>>,
    recv: ReadData,
    file_pos: &AtomicU64,
    file_size: u64,
    outdated_pos: &AtomicU64,
    outdated_map: &FxHashMap<u64, MSec>,
    ttl: u64,
) {
    let pos = match get_index(index, recv.hash, file_pos, file_size, outdated_pos) {
        Some(pos) => pos,
        _ => {
            let _ = recv.sender.send(None);
            return;
        }
    };
    let mut time = MSec::now().sub(ttl);
    if let Some(outdated) = outdated_map.get(&recv.type_id) {
        if time.less_than(*outdated) {
            time = *outdated;
        }
    }
    match read(
        file,
        pos % file_size,
        recv.hash,
        recv.type_id,
        recv.estimate,
        time,
    )
    .await
    {
        Ok(buf) => {
            remove_index(index, recv.hash);
            let _ = recv.sender.send(Some(buf.to_vec()));
        }
        Err(err) if err.is::<OlderThanTtl>() => {
            if u40_less_than(outdated_pos.load(Ordering::Relaxed), pos) {
                outdated_pos.store(pos, Ordering::Relaxed);
            }
            let _ = recv.sender.send(None);
        }
        Err(err) => {
            error!("{}", err);
            let _ = recv.sender.send(None);
        }
    }
}

fn init_index(index: &Arc<ArcSwapOption<Vec<AtomicU64>>>, index_size: usize) {
    if index.load().is_none() {
        let mut vec = Vec::with_capacity(index_size);
        for _i in 0..index_size {
            vec.push(AtomicU64::new(0));
        }
        index.store(Some(Arc::new(vec)));
    }
}

fn remove_index(index: &Arc<ArcSwapOption<Vec<AtomicU64>>>, hash: u128) {
    let index = index.load();
    if let Some(index) = index.as_ref() {
        let index_mask = index.len() as u64 - 1;
        let hash_idx = (hash as u64) & index_mask;
        for i in 0..SET_ASSOCIATIVE {
            let candidate = index[((hash_idx + i) & index_mask) as usize].load(Ordering::Relaxed);
            if (candidate & !POS_MASK) == ((hash as u64) & !POS_MASK) {
                index[((hash_idx + i) & index_mask) as usize].store(0, Ordering::Relaxed);
                break;
            }
        }
    }
}

fn set_index(index: &Arc<ArcSwapOption<Vec<AtomicU64>>>, hash: u128, file_pos: u64) {
    let index = index.load();
    if let Some(index) = index.as_ref() {
        let index_mask = index.len() as u64 - 1;
        let hash_idx = (hash as u64) & index_mask;
        let mut idx = 0;
        let mut pos = index[hash_idx as usize].load(Ordering::Relaxed) & POS_MASK;
        for i in 0..SET_ASSOCIATIVE {
            let candidate = index[((hash_idx + i) & index_mask) as usize].load(Ordering::Relaxed);
            if candidate == 0 || (candidate & !POS_MASK) == ((hash as u64) & !POS_MASK) {
                idx = i;
                break;
            }
            if u40_less_than(candidate & POS_MASK, pos) {
                pos = candidate & POS_MASK;
                idx = i;
            }
        }
        let hash_pos = ((hash as u64) & !POS_MASK) | ((file_pos >> ALIGNMENT_SHIFT) & POS_MASK);
        index[((hash_idx + idx) & index_mask) as usize].store(hash_pos, Ordering::Relaxed);
    }
}

fn get_index(
    index: &Arc<ArcSwapOption<Vec<AtomicU64>>>,
    hash: u128,
    file_pos: &AtomicU64,
    file_size: u64,
    outdated_pos: &AtomicU64,
) -> Option<u64> {
    let index = index.load();
    if let Some(index) = index.as_ref() {
        let index_mask = index.len() as u64 - 1;
        let hash_idx = (hash as u64) & index_mask;
        for i in 0..SET_ASSOCIATIVE {
            let candidate = index[((hash_idx + i) & index_mask) as usize].load(Ordering::Relaxed);
            if (candidate & !POS_MASK) == ((hash as u64) & !POS_MASK) {
                let pos = (candidate & POS_MASK) << ALIGNMENT_SHIFT;
                if u40_less_than(outdated_pos.load(Ordering::Relaxed), pos)
                    && u40_less_than(
                        file_pos.load(Ordering::Relaxed).wrapping_sub(file_size),
                        pos,
                    )
                {
                    return Some(pos);
                } else {
                    return None;
                }
            }
        }
    }
    None
}

fn u40_less_than(lhs: u64, rhs: u64) -> bool {
    let lhs = lhs & 0xFFFFFFFFFF;
    let rhs = rhs & 0xFFFFFFFFFF;
    let lhs = lhs | ((lhs & 0x8000000000) * 0x1FFFFFE);
    let rhs = rhs | ((rhs & 0x8000000000) * 0x1FFFFFE);
    lhs.wrapping_sub(rhs) > u64::MAX / 2
}

async fn write(file: &File, mut pos: u64, mut buf: BytesMut) -> Result<()> {
    loop {
        let (res, _buf) = file.write_at(buf, pos).await;
        buf = _buf;
        let len = res?;
        if len == 0 {
            bail!("write zero byte error");
        }
        if buf.len() > len {
            pos += len as u64;
            buf.advance(len);
            continue;
        }
        break;
    }
    Ok(())
}

#[derive(Debug, derive_more::Display)]
struct OlderThanTtl;
impl std::error::Error for OlderThanTtl {}

async fn read(
    file: &File,
    pos: u64,
    hash: u128,
    type_id: u64,
    estimate: usize,
    time: MSec,
) -> Result<BytesMut> {
    let mut rest = HEADER_SIZE + estimate * 2;
    let mut buf = BytesMut::with_capacity(rest);
    loop {
        let vec = vec![0u8; rest];
        let (res, vec) = file.read_at(vec, pos + buf.len() as u64).await;
        let len = res?;
        if len == 0 {
            bail!("read zero byte error");
        }
        buf.put_slice(&vec[..len]);
        if buf.len() < HEADER_SIZE {
            continue;
        }
        let mut chunk = buf.chunk();
        if chunk.get_u128_le() != hash || chunk.get_u64_le() != type_id {
            bail!("hash or type_id error");
        }
        if MSec::from(chunk.get_u64_le()).less_than(time) {
            bail!(OlderThanTtl);
        }
        let len = chunk.get_u32_le() as usize;
        if buf.len() >= HEADER_SIZE + len {
            if chunk.get_u32_le() != checksum(&chunk[..len]) {
                bail!("checksum error");
            }
            buf.advance(HEADER_SIZE);
            buf.truncate(len);
            break;
        }
        rest = HEADER_SIZE + len - buf.len();
    }
    Ok(buf)
}

fn checksum(v: &[u8]) -> u32 {
    let h = fxhash::hash(v);
    (h >> 32 ^ h) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_u40_less_than() {
        assert_eq!(u40_less_than(1, 2), true);
        assert_eq!(u40_less_than(1, 1), false);
        assert_eq!(u40_less_than(2, 1), false);
        assert_eq!(u40_less_than(0xffffffffff, 0), true);
        assert_eq!(u40_less_than(0xfffffffffe, 0xffffffffff), true);
    }
    #[tokio::test]
    async fn test() {
        let cache = StorageCache::start("test".into(), 257, 1, 10000, 2).unwrap();
        let buf = BytesMut::from("datadatadatadatadatadatadatadatadatadatadatadatadata".as_bytes());
        for i in 1..10u32 {
            cache.write(
                fxhash::hash64(&i.to_le_bytes()) as u128,
                2,
                buf.chunk(),
                MSec::now(),
            );
        }
        thread::sleep(std::time::Duration::from_millis(10));
        // cache.invalidate_all_of(2);
        let result = cache
            .read(fxhash::hash64(&2u32.to_le_bytes()) as u128, 2, 10)
            .await;
        println!("1: {:?}", result);
        let result = cache
            .read(fxhash::hash64(&3u32.to_le_bytes()) as u128, 2, 10)
            .await;
        println!("2: {:?}", result);
        let result = cache
            .read(fxhash::hash64(&4u32.to_le_bytes()) as u128, 2, 10)
            .await;
        println!("3: {:?}", result);
        // println!("{:?}", buf);
        // assert_eq!(result, Some(buf));
    }
}
