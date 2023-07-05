//! Mod args
//!     CmdlineArgs: parse command line arguments for bench
//!     
//! Mod doorbell 
//! Support RDMA post_send/post_recv doorbell.
//!     1. RcDoorbellHelper: DoorbellHelper for RC READ/WRITE
//!     2. UdDoorbellHelper: DoorbellHelper for UD SEND
//!     3. RecvDoorbellHelper: DoorbellHelper for RECV
//! Mod ud_endpoint
//!     pub fn bootstrap_uds(
//!         socket: &mut TcpStream,
//!         nic_idx: usize,
//!         nic_num: usize,
//!         threads: usize,
//!         client_id: u64,
//!     ) -> (Vec<Arc<QueuePair>>, Vec<Arc<DatagramEndpoint>>)
//!     bootstrap UD connections at client-side
//!     
//!     pub fn bootstrap_ud_server(
//!         threads: usize,
//!         nic_idx: usize,
//!         nic_num: usize,
//!     ) -> (Vec<Arc<QueuePair>>, Vec<UdMeta>)
//!     bootstrap UD server, after calling, server should be ready for client to send
//! 
//! Mod rdtsc
//!     An x86-specific timer lib, should be banned with --features "ARM" in a ARM environment.
//! Mod doca
//!     CmdlineArgs: parse command line arguments for doca_related bench

#![feature(trusted_random_access)]

pub mod args;
pub mod doorbell;
pub mod ud_endpoint;
pub mod ud_manager;
pub mod ud_message;

#[cfg(not(feature = "ARM"))]
pub mod rdtsc;

pub mod doca;

pub const MIN_SERVER_LIFE: u32 = 30;
pub const MAX_CLIENTS: usize = 24;

/// maxium size of recv-batch posted
pub const MAX_RECV_NUM: usize = 64;
/// global route header sz for ud send
pub const GRH_SZ: u64 = 40;
/// maxium inline sz for a ud send
pub const MAX_INLINE_SZ: usize = 64;
/// maxium pending messages
pub const MAX_FLYING_MSG: u64 = 256;
/// maxium payload for a ud send/recv
pub const MAX_MSG_SZ: u64 = 4096;
/// cacheline sz
pub const CACHE_LINE_SZ: u64 = 64;

#[inline]
pub fn round_up(num: u64, factor: i64) -> u64
{
    if factor == 0 
    {
        return num;
    }

    ((num + factor as u64 - 1) as i64 & (-factor)) as u64
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_ud_align_to_cacheline() {
        use crate::ud_message::align_to_cacheline;
        let mut payload = 16;
        payload = align_to_cacheline(payload);
        assert_eq!(payload, 24);

        payload = 1024;
        payload = align_to_cacheline(payload);
        assert_eq!(payload, 984);
    }
}
