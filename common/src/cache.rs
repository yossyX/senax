pub mod db_cache;
pub mod fast_cache;
pub mod msec;

#[cfg(all(feature = "uring", target_os = "linux"))]
pub mod storage_cache;
#[cfg(not(all(feature = "uring", target_os = "linux")))]
#[path = "cache/storage_cache_dummy.rs"]
pub mod storage_cache;

pub trait CycleCounter {
    #[must_use]
    fn cycle_add(&self, rhs: Self) -> Self;
    /// <
    #[must_use]
    fn less_than(&self, rhs: Self) -> bool;
    /// >=
    #[must_use]
    fn greater_equal(&self, rhs: Self) -> bool;
}

impl CycleCounter for u32 {
    fn cycle_add(&self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
    fn less_than(&self, rhs: Self) -> bool {
        self.wrapping_sub(rhs) > Self::MAX / 2
    }
    fn greater_equal(&self, rhs: Self) -> bool {
        !self.less_than(rhs)
    }
}

impl CycleCounter for u64 {
    fn cycle_add(&self, rhs: Self) -> Self {
        self.wrapping_add(rhs)
    }
    fn less_than(&self, rhs: Self) -> bool {
        self.wrapping_sub(rhs) > Self::MAX / 2
    }
    fn greater_equal(&self, rhs: Self) -> bool {
        !self.less_than(rhs)
    }
}

/// assuming MiMalloc
pub fn calc_mem_size(size: usize) -> usize {
    if size >= 512 * 1024 {
        return (size + 4095) / 4096 * 4096;
    }
    let uintptr_t = std::mem::size_of::<usize>();
    let mut wsize: usize = (size + uintptr_t - 1) / uintptr_t;
    if wsize <= 1 {
        return 8;
    }
    if wsize <= 8 {
        return ((wsize + 1) & !1) * 8;
    }
    wsize -= 1;
    let b = uintptr_t * 8 - 1 - wsize.leading_zeros() as usize;
    let bin = ((b << 2) + ((wsize >> (b - 2)) & 0x03)) - 3;
    let rank = (bin + 3) / 4;
    (16 << rank) - (rank * 4 - bin) * (2 << rank)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test() {
        assert_eq!(1u32.cycle_add(1), 2);
        assert!(1u32.less_than(2));
        assert!(!2u32.less_than(2));
        assert!(!3u32.less_than(2));
        assert!(u32::MAX.less_than(1));
        assert!(0u32.cycle_add(u32::MAX).less_than(1.cycle_add(u32::MAX)));
        assert!(!1u32.cycle_add(u32::MAX).less_than(0.cycle_add(u32::MAX)));
        assert!(1u32.cycle_add(u32::MAX).less_than(2.cycle_add(u32::MAX)));
        assert!(!2u32.cycle_add(u32::MAX).less_than(1.cycle_add(u32::MAX)));
        assert!(0u32.cycle_add(u32::MAX).less_than(2.cycle_add(u32::MAX)));
        assert!(!2u32.cycle_add(u32::MAX).less_than(0.cycle_add(u32::MAX)));
    }

    #[test]
    fn test_mem_size() {
        assert_eq!(calc_mem_size(15), 16);
        assert_eq!(calc_mem_size(16), 16);
        assert_eq!(calc_mem_size(17), 32);
        assert_eq!(calc_mem_size(31), 32);
        assert_eq!(calc_mem_size(32), 32);
        assert_eq!(calc_mem_size(33), 48);
        assert_eq!(calc_mem_size(47), 48);
        assert_eq!(calc_mem_size(48), 48);
        assert_eq!(calc_mem_size(49), 64);
        assert_eq!(calc_mem_size(63), 64);
        assert_eq!(calc_mem_size(64), 64);
        assert_eq!(calc_mem_size(65), 80);
        assert_eq!(calc_mem_size(79), 80);
        assert_eq!(calc_mem_size(80), 80);
        assert_eq!(calc_mem_size(81), 96);
        assert_eq!(calc_mem_size(95), 96);
        assert_eq!(calc_mem_size(96), 96);
        assert_eq!(calc_mem_size(97), 112);
        assert_eq!(calc_mem_size(111), 112);
        assert_eq!(calc_mem_size(112), 112);
        assert_eq!(calc_mem_size(113), 128);
        assert_eq!(calc_mem_size(128), 128);
        assert_eq!(calc_mem_size(129), 160);
        assert_eq!(calc_mem_size(160), 160);
        assert_eq!(calc_mem_size(161), 192);
        assert_eq!(calc_mem_size(192), 192);
        assert_eq!(calc_mem_size(193), 224);
        assert_eq!(calc_mem_size(224), 224);
        assert_eq!(calc_mem_size(225), 256);
        assert_eq!(calc_mem_size(256), 256);
        assert_eq!(calc_mem_size(257), 320);
        assert_eq!(calc_mem_size(320), 320);
        assert_eq!(calc_mem_size(321), 384);
        assert_eq!(calc_mem_size(384), 384);
        assert_eq!(calc_mem_size(385), 448);
        assert_eq!(calc_mem_size(448), 448);
        assert_eq!(calc_mem_size(449), 512);
        assert_eq!(calc_mem_size(7169), 8192);
        assert_eq!(calc_mem_size(524288), 524288);
        assert_eq!(calc_mem_size(524289), 528384);
    }
}
