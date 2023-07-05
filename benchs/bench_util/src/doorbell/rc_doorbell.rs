use KRdmaKit::{MemoryRegion, DatapathError};
use KRdmaKit::queue_pairs::QueuePair;
use KRdmaKit::rdma_shim::bindings::*;

use crate::doorbell::DoorbellHelper;

use core::iter::TrustedRandomAccessNoCoerce;
use core::ops::Range;

use std::sync::Arc;

pub struct RcDoorbellHelper {
    send_doorbell: DoorbellHelper,
    send_qp: Arc<QueuePair>,
}

impl RcDoorbellHelper {
    pub fn create(capacity: usize, qp: Arc<QueuePair>) -> Self {
        Self {
            send_doorbell: DoorbellHelper::new(capacity),
            send_qp: qp,
        }
    }

    ///Init RcDoorbellHelper's internal doorbell with specific IBV_WR_OPCODE
    /// Since one-sided test may read or write, 
    /// we leave the init(op) to be called by user to delay initialization.
    #[inline]
    pub fn init(&mut self, op: u32) {
        self.send_doorbell.init(op);
    }

    ///Post WR to `send_doorbell`'s next entry
    /// If `send_doorbell` is full, 
    /// this func will call flush_doorbell() to send all batched WRs.
    pub fn post_send(
        &mut self,
        mr: &MemoryRegion,
        range: Range<u64>,
        signaled: bool,
        raddr: u64,
        rkey: u32,
        wr_id: u64
    ) -> Result<(), DatapathError> {
        self.send_doorbell.next();
        /* set sge for current wr */
        self.send_doorbell.cur_sge().addr = unsafe { mr.get_rdma_addr() + range.start };
        self.send_doorbell.cur_sge().length = range.size() as u32;
        self.send_doorbell.cur_sge().lkey = mr.lkey().0;

        /* set wr fields */
        let send_flag: i32 = if signaled { ibv_send_flags::IBV_SEND_SIGNALED as i32 } else { 0 };
        self.send_doorbell.cur_wr().wr_id = wr_id;

        #[cfg(feature = "OFED_5_4")]
        {
            self.send_doorbell.cur_wr().send_flags = send_flag as u32;
        }

        #[cfg(not(feature = "OFED_5_4"))]
        {
            self.send_doorbell.cur_wr().send_flags = send_flag;
        }
        unsafe {
            self.send_doorbell.cur_wr().wr.rdma.as_mut().remote_addr = raddr;
            self.send_doorbell.cur_wr().wr.rdma.as_mut().rkey = rkey;
        }
        // no need to set imm_data for read/write

        let mut res = Ok(());
        if self.send_doorbell.is_full() {
            // flush a doorbell
            self.send_doorbell.freeze();
            res = self.flush_doorbell();
            self.send_doorbell.clear();
        }
        res
    }

    #[inline]
    pub fn flush_doorbell(&mut self) -> Result<(), DatapathError> {
        self.send_qp.post_send_wr(self.send_doorbell.first_wr_ptr())
    }
}