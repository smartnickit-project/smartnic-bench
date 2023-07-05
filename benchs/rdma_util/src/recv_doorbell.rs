use std::sync::Arc;
use KRdmaKit::{MemoryRegion, QueuePair, DatapathError};
use KRdmaKit::rdma_shim::bindings::*;
use core::iter::TrustedRandomAccessNoCoerce;
use core::ops::Range;

pub(self) const DEFAULT_BATCH_SZ: usize = 64;

pub const MAX_RECV_NUM: usize = 64;

/* A structure to help recv doorbell. */
pub struct RecvDoorbell {
    /// contains recv wrs and sges to save requests
    pub wrs: [ibv_recv_wr; DEFAULT_BATCH_SZ],
    pub sges: [ibv_sge; DEFAULT_BATCH_SZ],

    pub cur_idx: isize,
    pub capacity: usize,
}

impl RecvDoorbell {
    pub fn create(capacity: usize) -> Self {
        let ret = Self {
            capacity,
            cur_idx: -1,
            wrs: [Default::default(); DEFAULT_BATCH_SZ],
            sges: [ibv_sge {
                addr: 0,
                length: 0,
                lkey: 0,
            }; DEFAULT_BATCH_SZ],
        };
        ret
    }

    #[inline]
    pub fn init(&mut self) {
        for i in 0..self.capacity {
            self.wrs[i].num_sge = 1;
            self.wrs[i].next = &mut self.wrs[(i + 1) % self.capacity] as *mut ibv_recv_wr;
            self.wrs[i].sg_list = &mut self.sges[i] as *mut ibv_sge;
        }
    }

    /// Return current batching size
    #[inline]
    pub fn size(&self) -> isize {
        self.cur_idx + 1
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size() <= 0
    }
    #[inline]
    pub fn empty(&mut self) {
        self.cur_idx = -1;
    }
    #[inline]
    pub fn is_full(&self) -> bool {
        self.size() >= self.capacity as isize
    }

    /// Get the next doorbell entry
    /// # Return value
    /// - `true` means the doorbell batching size is less than `capacity`, it is ok to add a new doorbell
    /// - `false` means doorbell is full, cannot add new entry
    /// 
    /// User shall check its return value
    #[inline]
    pub fn next(&mut self) -> bool {
        if self.is_full() {
            return false;
        }
        self.cur_idx += 1;
        true
    }
}

impl RecvDoorbell {
    /// Before flushing the doorbell, we must freeze it to prevent adding
    #[inline]
    pub fn freeze(&mut self) {
        assert!(!self.is_empty()); // should not be empty
        self.cur_wr().next = core::ptr::null_mut();
    }

    /// After flushing the doorbell, unfreeze it
    #[inline]
    pub fn freeze_done(&mut self) {
        assert!(!self.is_empty());
        if self.cur_idx == (self.capacity - 1) as isize {
            self.wrs[self.cur_idx as usize].next = &mut self.wrs[0] as *mut ib_recv_wr;
        } else {
            self.wrs[self.cur_idx as usize].next =
                &mut self.wrs[(self.cur_idx + 1) as usize] as *mut ib_recv_wr;
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.freeze_done();
        self.cur_idx = -1;
    }
    /// Return the ptr to current doorbell entry's wr
    #[inline]
    pub fn cur_wr(&mut self) -> &mut ibv_recv_wr {
        return if self.is_empty() {
            &mut self.wrs[0]
        } else {
            &mut self.wrs[self.cur_idx as usize]
        };
    }
    /// Return the ptr to current doorbell entry's sge
    #[inline]
    pub fn cur_sge(&mut self) -> &mut ibv_sge {
        return if self.is_empty() {
            &mut self.sges[0]
        } else {
            &mut self.sges[self.cur_idx as usize]
        };
    }

    #[inline]
    pub fn first_wr_ptr(&mut self) -> *mut ib_recv_wr {
        &mut self.wrs[0] as *mut ibv_recv_wr
    }
    /// Return the ptr to specified doorbell entry's wr
    ///**WRRN**: No check for idx. The caller has to take care of it by himself
    #[inline]
    pub fn get_wr_ptr(&mut self, idx: usize) -> *mut ibv_recv_wr {
        &mut self.wrs[idx] as *mut ibv_recv_wr
    }
    /// Return the ptr to specified doorbell entry's sge
    #[inline]
    pub fn get_sge_ptr(&mut self, idx: usize) -> *mut ibv_sge {
        &mut self.sges[idx] as *mut ibv_sge
    }
}

/* Maintain recv requests with a doorbell 
    Capacity of the doorbell is designated by the `capacity` arg in RecvDoorbellHelper::create
*/
pub struct RecvDoorbellHelper {
    recv_doorbell: RecvDoorbell,
    recv_qp: Arc<QueuePair>,
}

impl RecvDoorbellHelper {
    pub fn create(capacity: usize, qp: Arc<QueuePair> ) -> Self {
    
        let mut ret = Self {
            recv_doorbell: RecvDoorbell::create(capacity),
            recv_qp: qp,
        };
        ret.recv_doorbell.init();
        ret
    }

    #[inline]
    pub fn sanity_check(&self) -> bool {
        let mut ret = true;
        for i in 0..self.recv_doorbell.capacity {
            let sge_ptr = & self.recv_doorbell.sges[i] as *const ibv_sge;
            let wr_sg_list = self.recv_doorbell.wrs[i].sg_list;
            ret = sge_ptr as u64 == wr_sg_list as u64;
        }
        ret
    }

    pub fn post_recv(
        &mut self,
        mr: &MemoryRegion,
        range: Range<u64>,
        wr_id: u64,
    ) -> Result<(), DatapathError> {
        self.recv_doorbell.next();
        // setup sge fields
        self.recv_doorbell.cur_sge().addr = unsafe { mr.get_rdma_addr() + range.start };
        self.recv_doorbell.cur_sge().length = range.size() as u32;
        self.recv_doorbell.cur_sge().lkey = mr.lkey().0;
        // setup recv wr fields
        self.recv_doorbell.cur_wr().wr_id = wr_id;

        let mut res = Ok(());
        if self.recv_doorbell.is_full() {
            // println!("flush a recv doorbell");
            self.recv_doorbell.freeze();
            res = self.flush();
            self.recv_doorbell.clear();
        }
        res
    }
    
    #[inline]
    pub fn flush(&mut self) -> Result<(), DatapathError> {
        self.recv_qp.post_recv_wr(self.recv_doorbell.first_wr_ptr())
    }
}