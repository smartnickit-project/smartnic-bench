use std::sync::Arc;
use KRdmaKit::{ MemoryRegion, QueuePair, DatapathError, DatagramEndpoint };
use KRdmaKit::rdma_shim::bindings::*;
use core::iter::TrustedRandomAccessNoCoerce;
use core::ops::Range;

use crate::doorbell::DoorbellHelper;
use crate::MAX_INLINE_SZ;

pub struct UdDoorbellHelper {
    send_doorbell: DoorbellHelper,
    send_qp: Arc<QueuePair>,
}

impl UdDoorbellHelper {
    pub fn create(capacity: usize, op: u32, qp: Arc<QueuePair>) -> Self {
        let mut ret = Self {
            send_doorbell: DoorbellHelper::new(capacity),
            send_qp: qp,
        };
        ret.send_doorbell.init(op);
        ret
    }

    #[inline]
    pub fn sanity_check(&self) -> bool {
        let mut ret = true;
        for i in 0..self.send_doorbell.capacity {
            let sge_ptr = &self.send_doorbell.sges[i] as *const ibv_sge;
            let wr_sg_list = self.send_doorbell.wrs[i].sg_list;
            ret &= (sge_ptr as u64) == (wr_sg_list as u64);
        }
        ret
    }

    pub fn post_send(
        &mut self,
        endpoint: &DatagramEndpoint,
        mr: &MemoryRegion,
        range: Range<u64>,
        wr_id: u64,
        imm_data: Option<u32>,
        signaled: bool
    ) -> Result<(), DatapathError> {
        self.send_doorbell.next();
        // setup sge fields
        self.send_doorbell.cur_sge().addr = unsafe { mr.get_rdma_addr() + range.start };
        self.send_doorbell.cur_sge().length = range.size() as u32;
        self.send_doorbell.cur_sge().lkey = mr.lkey().0;
        // setup UD SEND wr fields
        unsafe {
            self.send_doorbell.cur_wr().wr.ud.as_mut().remote_qpn = endpoint.qpn();
            self.send_doorbell.cur_wr().wr.ud.as_mut().remote_qkey = endpoint.qkey();
            self.send_doorbell.cur_wr().wr.ud.as_mut().ah = endpoint
                .raw_address_handler_ptr()
                .as_ptr();
        }
        self.send_doorbell.cur_wr().send_flags = match signaled {
            true => ibv_send_flags::IBV_SEND_SIGNALED.try_into().unwrap(),
            false => 0,
        };
        self.send_doorbell.cur_wr().send_flags |= if range.size() <= MAX_INLINE_SZ {
            ibv_send_flags::IBV_SEND_INLINE.try_into().unwrap()
        } else {
            0
        };

        let imm = match imm_data {
            Some(i) => i,
            None => 0,
        };

        #[cfg(feature = "OFED_5_4")]
        unsafe {
            *self.send_doorbell.cur_wr().__bindgen_anon_1.imm_data.as_mut() = imm;
        }
        #[cfg(not(feature = "OFED_5_4"))]
        {
            self.send_doorbell.cur_wr().imm_data = imm;
        }

        // info!("doorbell cur idx: {}, sansity: {}", self.send_doorbell.cur_idx, self.sanity_check());

        let mut res = Ok(());
        if self.send_doorbell.is_full() {
            // info!("doorbell is full");
            self.send_doorbell.freeze();
            res = self.flush();
            self.send_doorbell.clear();
        }
        res
    }

    #[inline]
    pub fn flush(&mut self) -> Result<(), DatapathError> {
        self.send_qp.post_send_wr(self.send_doorbell.first_wr_ptr())
    }
}