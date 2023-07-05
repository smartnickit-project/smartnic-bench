use std::sync::{ Arc };
use std::net::SocketAddr;

use clap::{ Command, arg, Parser };

use KRdmaKit::{ MemoryRegion, QueuePair, QueuePairBuilder, QueuePairStatus, UDriver, DatapathError };
use KRdmaKit::services_user::MRInfo;

use rand::RngCore;
use rand_chacha::ChaCha8Rng;

use crate::CACHE_LINE_SZ;
use crate::round_up;

#[derive(Debug, Parser)]
pub struct CmdlineArgs {
    /* Common fields of client and server */

    /// Number of NIC devices used
    #[arg(long, default_value_t = 1)]
    pub nic_num: usize,

    /// Index of the first NIC device used
    #[arg(long, default_value_t = 0)]
    pub nic_idx: usize,

    /// Memory region bytes of the server
    #[arg(long, default_value_t = 10240)]
    pub random_space: u64,

    /// The life of the bench (seconds)
    #[arg(long, default_value_t = 15)]
    pub life: u32,

    /// The listening address of server
    #[arg(long, default_value_t = String::from("127.0.0.1:8888"))]
    pub listen_addr: String,

    /// The reporting address of server
    #[arg(long, default_value_t = String::from("127.0.0.1:10001"))]
    pub report_addr: String,
    
    /* Client-specific fields */
    /// Number of threads used
    #[arg(short, long, default_value_t = 1)]
    pub threads: u64,

    /// Number of requests in a batch
    #[arg(short, long, default_value_t = 64)]
    pub factor: u64,

    /// Payload of each request
    #[arg(short, long, default_value_t = 64)]
    pub payload: u64,

    /// Client-local memory region bytes
    #[arg(long, default_value_t = 4096)]
    pub local_mr: u64,

    /// Whether to run READ bench
    #[arg(long)]
    pub read: bool,

    /// Whether to separate thread access area
    #[arg(long)]
    pub fixed: bool,

    /// Client id, which will be used to generate unique seed
    #[arg(long, default_value_t = 0)]
    pub client_id: u64,

    /// The random access area bytes
    #[arg(long, default_value_t = 1024)]
    pub thread_gap: u64,

    /// Whether to run lantency test
    #[arg(long)]
    pub latency_test: bool,

    /// One signal in <signal_size> requests
    #[arg(long, default_value_t = 16)]
    pub signal_size: usize,

    /// One doorbell for <db_size> requests
    #[arg(long, default_value_t = 16)]
    pub db_size: usize,

    /// Whether to report to <report_addr>
    #[arg(long)]
    pub report: bool,

    /// Whether to generate a signal at client for every request
    #[arg(long)]
    pub signaled: bool,

    /// Whether to apply doorbell batching
    #[arg(long)]
    pub doorbell: bool,

    /// Whether to profile the posting cost
    #[arg(long)]
    pub profile: bool,
    /* Server-specific fields */

    /// Whether to run the bench in server mode
    #[arg(long)]
    pub server: bool,

    /// Whether to allcate memory regions using huge pages
    #[arg(long)]
    pub huge_page: bool,
}

impl Clone for CmdlineArgs {
    fn clone(&self) -> Self {
        Self {
            listen_addr: self.listen_addr.clone(),
            report_addr: self.report_addr.clone(),
            ..*self
        }
    }
}

impl CmdlineArgs {
    /// coordinate the arguments to make them be compatible to each other
    pub fn coordinate(&mut self) {
        self.local_mr = std::cmp::max(self.factor * self.payload, self.local_mr);
        self.thread_gap = std::cmp::max(self.payload, self.thread_gap);
        self.random_space = std::cmp::max(self.payload, self.random_space);
        self.random_space = std::cmp::max(self.threads * self.thread_gap, self.random_space);
    }

    pub fn create_rc(&self, thread_id: usize) -> Result<(Arc<QueuePair>, Arc<MemoryRegion>, MRInfo), ()> {
        let addr: SocketAddr = self.listen_addr.parse().unwrap();
        let client_port: u8 = 1;
        let ctx = UDriver::create()
            .expect("failed to query device")
            .devices()
            .get((thread_id % self.nic_num) + self.nic_idx)
            .expect("no rdma device available")
            .open_context()
            .expect("failed to create RDMA context");
        let mut builder = QueuePairBuilder::new(&ctx);
        builder.allow_remote_rw().allow_remote_atomic().set_port_num(client_port);
        let qp = builder.build_rc().expect("failed to create the client QP");
        let qp = qp.handshake(addr).expect("Handshake failed!");
        let a = qp.status().expect("Query status failed!");
        match a {
            QueuePairStatus::ReadyToSend => {}
            _ => {
                return Err(());
            }
        }
        let mr_infos = qp.query_mr_info().expect("Failed to query MR info");
        let mr_metadata = mr_infos.inner().get("MR").expect("Unregistered MR");
        let client_mr = match self.huge_page {
            true => {
                Arc::new(MemoryRegion::new_huge_page(ctx.clone(), self.local_mr as _).expect(
                    "Failed to allocate hugepage MR for send buffer"
                ))
            },
            false => {
                Arc::new(MemoryRegion::new(ctx.clone(), self.local_mr as _).expect(
                    "Failed to allocate MR"
                ))
            }
        };
        
        
        let mr_buf = client_mr.get_virt_addr() as *mut u64;
        unsafe {
            *mr_buf = 0;
        }
        Ok((qp, client_mr, MRInfo {addr: mr_metadata.addr, capacity: mr_metadata.capacity, rkey: mr_metadata.rkey}))
    }

    pub fn get_next_index(&self, thread_idx: usize, rand: &mut ChaCha8Rng) -> u64 {
        let mut r = rand.next_u64();

        if self.payload == self.random_space {
            return 0;
        }

        if self.fixed {
            if self.thread_gap != 0 {
                // r = (thread_idx * 64) as _;
                assert!(self.thread_gap >= self.payload);
                assert!(self.threads * self.thread_gap <= self.random_space);
                r = (r % self.thread_gap) + (thread_idx as u64) * self.thread_gap;
            } else {
                r = 0;
            }
        }

        // align
        r = round_up(r, CACHE_LINE_SZ as i64);
        assert_eq!(r % CACHE_LINE_SZ, 0);

        let index = (r % (self.random_space - self.payload)) as u64;
        index
    }
}