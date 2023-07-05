use std::sync::{ Arc };

use clap::{ Command, arg, Arg, ArgAction, Parser };

use rand::RngCore;
use rand_chacha::ChaCha8Rng;

use crate::round_up;
use crate::CACHE_LINE_SZ;

#[derive(Parser)]
pub struct CmdlineArgs {
    /* Common fields of client and server */
    
    /// The PCIe device list (we suggest using one)
    #[arg(short, long)]
    pub pci_dev: Vec<String>,

    /// Memory region bytes of the server
    #[arg(long, default_value_t = 10 * 1024)]
    pub random_space: u64,

    /// The life of the bench (seconds)
    #[arg(long, default_value_t = 15)]
    pub life: u32,

    /// The listening address of server
    #[arg(long)]
    pub listen_addr: String,

    /// Whether to allcate memory regions using huge pages
    #[arg(long)]
    pub huge_page: bool,
    
    /* Client-specific fields */
    /// Client id, which will be used to generate unique seed
    #[arg(short, long, default_value_t = 0)]
    pub client_id: u64,

    /// Number of threads used
    #[arg(short, long, default_value_t = 1)]
    pub threads: u64,

    /// Payload of each request
    #[arg(long, default_value_t = 32)]
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

    /// The random access area bytes
    #[arg(long, default_value_t = 8192)]
    pub thread_gap: u64,

    /// Whether to run lantency test
    #[arg(long)]
    pub latency_test: bool,

    /// Number of requests in a batch
    #[arg(long, default_value_t = 64)]
    pub batch_size: usize,

    /* Server-specific fields */
    /// Whether to run bench as the server
    #[arg(long)]
    pub server: bool,
}

impl Clone for CmdlineArgs {
    fn clone(&self) -> Self {
        Self {
            pci_dev: self.pci_dev.clone(),
            listen_addr: self.listen_addr.clone(),
            ..*self
        }
    }
}

impl CmdlineArgs {
    /// coordinate the arguments to make them consistent with each other
    pub fn coordinate(&mut self) {
        self.local_mr = std::cmp::max(self.batch_size as u64 * self.payload, self.local_mr);
        self.thread_gap = std::cmp::max(self.payload, self.thread_gap);
        self.random_space = std::cmp::max(self.payload, self.random_space);
        self.random_space = std::cmp::max(self.threads * self.thread_gap, self.random_space);
    }

    /// get next index to access in the random region
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