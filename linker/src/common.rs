use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;
use std::sync::atomic::AtomicU64;
use tokio_uring::buf::{IoBuf, IoBufMut};

pub const ALPN_QUIC_HTTP: &[&[u8]] = &[b"hq-29"];
pub const LINKER_VER: u16 = 1;
pub const CMD_RESET: u16 = 1;
pub const SENDER: u16 = 0;
pub const RECEIVER: u16 = 1;
pub const ZSTD_LEVEL: i32 = 1;
pub const CONNECTION_SUCCESS: u8 = 0;
pub const LINKER_VER_ERROR: u8 = 1;
pub const PASSWORD_ERROR: u8 = 2;

pub static CONN_NO: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct Pack {
    pub data: Bytes, // including self length
    pub conn_no: u64,
    pub db: u64,
}

pub struct IoBytesMut(pub BytesMut, usize, usize);
impl IoBytesMut {
    pub fn new(capacity: usize) -> Self {
        Self(BytesMut::with_capacity(capacity), 0, capacity)
    }
    pub fn advance(&mut self, cnt: usize) {
        self.1 += cnt;
    }
    pub fn get(self) -> BytesMut {
        self.0
    }
    pub fn put_length(&mut self, n: u64) {
        self.0.put_u64_le(n);
        self.advance(8);
    }
}
unsafe impl IoBuf for IoBytesMut {
    fn stable_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        self.0.len()
    }

    fn bytes_total(&self) -> usize {
        self.2 - self.1
    }
}
unsafe impl IoBufMut for IoBytesMut {
    fn stable_mut_ptr(&mut self) -> *mut u8 {
        unsafe { self.0.as_mut_ptr().add(self.1) }
    }

    unsafe fn set_init(&mut self, init_len: usize) {
        if self.0.len() < init_len + self.1 {
            self.0.set_len(init_len + self.1);
        }
    }
}
