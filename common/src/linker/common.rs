use bytes::BytesMut;
use tokio_uring::buf::{IoBuf, IoBufMut};

pub const LINKER_VER: u16 = 1;
pub const SENDER: u16 = 0;
pub const RECEIVER: u16 = 1;

pub(crate) struct IoBytesMut(BytesMut, usize, usize);
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
            unsafe {
                self.0.set_len(init_len + self.1);
            }
        }
    }
}
