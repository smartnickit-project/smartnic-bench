use std::sync::atomic::{ compiler_fence, Ordering };
use std::slice;
use std::net::{ SocketAddr, TcpStream };
use std::io::Write;
use std::time::Duration;
use std::ptr::{ NonNull, null_mut };
use std::sync::Arc;

use bench_util::doca::args::CmdlineArgs;
use crate::bootstrap::*;
use bench_util::round_up;

use netbencher_core::*;
use log::info;

use doca::dma::DOCAContext;
use doca::{ DOCAMmap, DOCARegisteredMemory, BufferInventory, DOCAWorkQueue, DMAEngine, RawPointer, RawPointerMsg };

use nix::libc::*;

use crate::bootstrap::connection::*;

fn open_doca_device(pci_devs: &Vec<String>) -> (Arc<DOCAMmap>, usize) {
    let num_dev = pci_devs.len();
    let mut local_mmap = DOCAMmap::new().unwrap();

    for d in pci_devs.iter() {
        let device = doca::device::open_device_with_pci(d.as_str()).unwrap();
        let dev_idx = local_mmap.add_device(&device).unwrap();
    }
    /* populate the buffer info to mmap */
    (Arc::new(local_mmap), num_dev)
}


fn send_doca_config(addr: SocketAddr, num_dev: usize, mut doca_mmap: Arc<DOCAMmap>, src_buf: RawPointer) {
    let mut stream = TcpStream::connect(addr).unwrap();
    let mut doca_conn: DocaConnInfo = Default::default();

    for i in 0..num_dev {
        let export_desc = unsafe {
            Arc::get_mut_unchecked(&mut doca_mmap).export(i).unwrap()
        };
        doca_conn.exports.push(unsafe {
            slice::from_raw_parts_mut(export_desc.inner.as_ptr() as *mut _, export_desc.payload).to_vec()
        });
    }
    doca_conn.buffers.push(src_buf);
    stream.write(DocaConnInfo::serialize(doca_conn).as_slice()).unwrap();
}

pub fn perform_server_routine<T>(runner: Arc<BenchRunner<T>>, args: CmdlineArgs)
    where T: Send + 'static + Sync + Copy
{
    for i in 0..args.pci_dev.len() {
        println!("pcie dev 0: {}", &args.pci_dev[i]);
    }
    /* allocate local memory region */
    let mut src_buffer = vec![0u8; args.random_space as usize].into_boxed_slice();
    let src_region = RawPointer {
        inner: match args.huge_page {
            false => {
                NonNull::new(src_buffer.as_mut_ptr() as *mut _).unwrap()
            }
            true => {
                let capacity = round_up(args.random_space, 2 << 20);
                let data = unsafe {
                    mmap(
                        null_mut(),
                        capacity as size_t,
                        PROT_READ | PROT_WRITE,
                        MAP_PRIVATE | MAP_ANONYMOUS | MAP_POPULATE | MAP_HUGETLB,
                        -1,
                        0
                    )
                };

                if data == MAP_FAILED {
                    panic!("Failed to create huge-page MR");
                }
                NonNull::new(data).unwrap()
            }
        },
        payload: args.random_space as usize,
    };

    /* open all doca devices specified by the user and register the host memory region */
    let (local_mmap, num_dev) = open_doca_device(&args.pci_dev);
    local_mmap.populate(src_region).unwrap();

    /* and send the export_desc and src_buffer to dpu */
    send_doca_config(
        args.listen_addr.parse().unwrap(), 
        num_dev, 
        local_mmap.clone(), 
        src_region
    );
    
    /* keep the server alive until runner stop */
    while runner.running() {
        compiler_fence(Ordering::SeqCst);
    }

    // unmap/dealloc the buffer
    if args.huge_page {
        // unmap hugepages
    } else {
        // dealloc normal memory pages
    }
    
    info!("Server exit.");
}