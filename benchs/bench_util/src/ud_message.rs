use crate::{ GRH_SZ, CACHE_LINE_SZ };
use crate::round_up;

static MSG_SCALE: f64 = 3.3;

/// Align every UD message to cache
/// First align with cacheline, then minus GRH_SZ
/// to make sure no partial write will be generated at remote side.
#[inline]
pub fn align_to_cacheline(payload: u64) -> u64 {
    let mut result = payload;
    if payload <= CACHE_LINE_SZ {
        result = CACHE_LINE_SZ  - GRH_SZ;
    } else {
        let payload = round_up(payload, CACHE_LINE_SZ.try_into().unwrap());
        result = payload - GRH_SZ;
    }
    result
}

pub struct UdBuffer {
    pub capacity: u64,
    pub cur_idx: u64,
    pub msg_size: u64,
}

impl UdBuffer {
    pub fn new(capacity: u64, msg_size: u64) -> Self {
        Self {
            capacity: capacity,
            cur_idx: 0,
            msg_size: msg_size,
        }
    }

    #[inline]
    pub fn get_region_size(&self) -> u64 {
        round_up(self.capacity * self.msg_size * 4, 64)
    }

    #[inline]
    pub fn get_start_addr(&mut self) -> u64 {
        let start = round_up(((self.cur_idx * self.msg_size) as f64 * MSG_SCALE) as u64, 64);
        self.cur_idx = (self.cur_idx + 1) % self.capacity;
        start
    }
}