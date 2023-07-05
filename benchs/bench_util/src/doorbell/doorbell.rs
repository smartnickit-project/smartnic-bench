use KRdmaKit::rdma_shim::bindings::*;

/// We hardcoded the maximum batch size.
/// Typically, NIC doesn't expect a very large batch size.
pub const MAX_BATCH_SZ: usize = 64;

///A struct to help send doorbell.
/// It contains wrs and sges to save requests
/// 
pub struct DoorbellHelper {
    pub wrs: [ibv_send_wr; MAX_BATCH_SZ],
    pub sges: [ibv_sge; MAX_BATCH_SZ],
    pub capacity: usize,
    cur_idx: isize,
}

impl DoorbellHelper {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            cur_idx: -1,
            wrs: [Default::default(); MAX_BATCH_SZ],
            sges: [ibv_sge {
                addr: 0,
                length: 0,
                lkey: 0,
            }; MAX_BATCH_SZ],
        }
    }

    /// Create a DoorbellHelp, and initailize all its wrs and sges
    /// # Arguments
    /// - `capacity` is the max batch size of the doorbll
    /// - `op` is the ib operation shared by all entries in this doorbell
    pub fn create(capacity: usize, op: u32) -> Self {
        let mut ret = Self {
            capacity,
            cur_idx: -1,
            wrs: [Default::default(); MAX_BATCH_SZ],
            sges: [ibv_sge {
                addr: 0,
                length: 0,
                lkey: 0,
            }; MAX_BATCH_SZ],
        };
        ret.init(op);
        ret
    }

    #[inline]
    pub fn init(&mut self, op: u32) {
        for i in 0..self.capacity {
            self.wrs[i].opcode = op;
            self.wrs[i].num_sge = 1;
            self.wrs[i].next = &mut self.wrs[(i + 1) % self.capacity] as *mut ibv_send_wr;
            self.wrs[i].sg_list = &mut self.sges[i] as *mut ibv_sge;
        }
    }

    #[inline]
    pub fn sanity_check(&self) -> bool {
        let mut ret = true;
        for i in 0..self.capacity {
            let sge_ptr = &(self.sges[i]) as *const ibv_sge;
            let wr_sg_list = self.wrs[i].sg_list;
            ret &= (sge_ptr as u64) == (wr_sg_list as u64);
        }
        ret
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

impl DoorbellHelper {
    // Before flushing the doorbell, we must freeze it to prevent adding
    #[inline]
    pub fn freeze(&mut self) {
        assert!(!self.is_empty()); // should not be empty
        self.cur_wr().next = core::ptr::null_mut();
    }

    // After flushing the doorbell, unfreeze it
    #[inline]
    pub fn freeze_done(&mut self) {
        assert!(!self.is_empty());
        if self.cur_idx == (self.capacity - 1) as isize {
            self.wrs[self.cur_idx as usize].next = &mut self.wrs[0] as *mut ib_send_wr;
        } else {
            self.wrs[self.cur_idx as usize].next =
                &mut self.wrs[(self.cur_idx + 1) as usize] as *mut ib_send_wr;
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.freeze_done();
        self.cur_idx = -1;
    }
    // Return the ptr to current doorbell entry's wr
    #[inline]
    pub fn cur_wr(&mut self) -> &mut ib_rdma_wr {
        return if self.is_empty() {
            &mut self.wrs[0]
        } else {
            &mut self.wrs[self.cur_idx as usize]
        };
    }
    // Return the ptr to current doorbell entry's sge
    #[inline]
    pub fn cur_sge(&mut self) -> &mut ibv_sge {
        return if self.is_empty() {
            &mut self.sges[0]
        } else {
            &mut self.sges[self.cur_idx as usize]
        };
    }

    #[inline]
    pub fn first_wr_ptr(&mut self) -> *mut ib_send_wr {
        &mut self.wrs[0] as *mut ibv_send_wr
    }
    // Return the ptr to specified doorbell entry's wr
    // **WRRN**: No check for idx. The caller has to take care of it by himself
    #[inline]
    pub fn get_wr_ptr(&mut self, idx: usize) -> *mut ib_rdma_wr {
        &mut self.wrs[idx] as *mut ibv_send_wr
    }
    // Return the ptr to specified doorbell entry's sge
    #[inline]
    pub fn get_sge_ptr(&mut self, idx: usize) -> *mut ibv_sge {
        &mut self.sges[idx] as *mut ibv_sge
    }
}
