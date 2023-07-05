use std::sync::Arc;
use std::ptr::{ NonNull, null_mut };
use std::time::Duration;
use std::net::SocketAddr;

use doca::open_device_with_pci;
use doca::dma::{ DOCAContext, DOCADMAJob };
use doca::{ DOCAError, RawPointer, RawPointerMsg, DOCAResult, LoadedInfo, DOCABuffer, DOCARegisteredMemory, DOCAMmap, BufferInventory, DOCAWorkQueue, DMAEngine };

use tokio::net::{ TcpListener };
use tokio::io::{ AsyncReadExt, AsyncWriteExt };
use tokio::time::timeout;
use tokio::runtime::Runtime;

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;

use bench_util::doca::args::CmdlineArgs;
use bench_util::round_up;

use crate::bootstrap::*;

use netbencher_core::*;

use nix::libc::*;

use log::*;

pub async fn recv_doca_config(addr: SocketAddr) -> Vec<u8> {
    let mut conn_info = [0u8; DOCA_MAX_CONN_LENGTH];
    let mut conn_info_len = 0;
    /* receive the DOCA buffer message from the host */
    let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        if let Ok(res) = timeout(Duration::from_secs(1), listener.accept()).await {
            let (mut stream, _) = res.unwrap();
            conn_info_len = stream.read(&mut conn_info).await.unwrap();
            break;
        }
    }

    conn_info[0..conn_info_len].to_vec()
}

fn load_doca_config(thread_id: usize, doca_conn: &DocaConnInfo) -> DOCAResult<LoadedInfo> {
    /* parse the received messages */
    let dev_id = thread_id % doca_conn.exports.len();
    let buf_id = thread_id % doca_conn.buffers.len();
    let mut export_desc_buffer = doca_conn.exports[dev_id].to_vec().into_boxed_slice();
    let export_payload = export_desc_buffer.len();
    Ok(LoadedInfo {
        export_desc: RawPointer {
            inner: NonNull::new(Box::into_raw(export_desc_buffer) as *mut _).unwrap(),
            payload: export_payload,
        },
        remote_addr: doca_conn.buffers[buf_id],
    })
}

#[inline]
fn post_dma_reqs<T>
(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    args: CmdlineArgs,
    mut workq: Arc<DOCAWorkQueue<DMAEngine>>,
    local_buf: DOCABuffer,
    remote_buf: DOCABuffer,
) 
    where T: Send + 'static + Sync + Copy
{
    let mut src_buf_len = 0;
    let mut dst_buf_len = 0;
    let (src_buf, dst_buf) = match args.read {
        true => {
            src_buf_len = args.random_space as usize;
            dst_buf_len = args.local_mr as usize;
            (remote_buf, local_buf)
        }
        false => {
            src_buf_len = args.local_mr as usize;
            dst_buf_len = args.random_space as usize;
            (local_buf, remote_buf)
        }
    };

    let mut dma_job = workq.create_dma_job(src_buf, dst_buf);

    /* the testing logic of  */
    let mut rand = ChaCha8Rng::seed_from_u64(
        ((0xdeadbeaf + 73 * thread_id) as u64) + args.client_id * 37
    );
    while runner.running() {
        let mut start = 0;
        /* post dma requests */
        for i in 0..args.batch_size {
            let (src_offset, dst_offset) = match args.read {
                true => {
                    (args.get_next_index(thread_id,&mut rand) as usize, start as usize)
                },
                false => {
                    (start as usize, args.get_next_index(thread_id,&mut rand) as usize)
                }
            };
            
            dma_job.set_src_data(src_offset, args.payload as usize);
            dma_job.set_dst_data(dst_offset, args.payload as usize);
            start += args.payload;
            unsafe {
                Arc::get_mut_unchecked(&mut workq).submit(&dma_job).expect("failed to submit the job");
            }
        }

        /* retrieve dma job results */
        for i in 0..args.batch_size {
            loop {
                let event = unsafe {
                    Arc::get_mut_unchecked(&mut workq).poll_completion()
                };
                match event {
                    Ok(_e) => {
                        break;
                    }
                    Err(e) => {
                        if e == DOCAError::DOCA_ERROR_AGAIN {
                            continue;
                        } else {
                            panic!("Job failed! {:?}", e);
                        }
                    }
                }
            }
        }

        unsafe {
            Arc::get_mut_unchecked(&mut stat).finished_batch_ops(args.batch_size.try_into().unwrap());
        }
    }
}

pub fn perform_client_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    stat: Arc<BenchStat>,
    conn: Vec<u8>,
    mut args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let doca_conn = DocaConnInfo::deserialize(conn.as_slice());
    let remote_config = load_doca_config(thread_id, &doca_conn).unwrap();
    debug!(
        "Check export len {}, remote len {}, remote addr {:?}",
        remote_config.export_desc.payload,
        remote_config.remote_addr.payload,
        remote_config.remote_addr.inner.as_ptr()
    );

    /* allocate local buffer */
    // FIXME: do we need larger buffer for more test scenes?
    args.local_mr = args.batch_size as u64 * args.payload;
    let mut local_buffer = vec![0u8; args.local_mr as usize].into_boxed_slice();
    let local_region = RawPointer {
        inner: match args.huge_page {
            false => {
                NonNull::new(local_buffer.as_mut_ptr() as *mut _).unwrap()
            }
            true => {
                let capacity = round_up(args.local_mr, 2 << 20);
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
        payload: args.local_mr as usize,
    };
    /* init DOCA core objects */
    let device = open_device_with_pci(args.pci_dev[0].as_str()).unwrap();
    let mut doca_mmap = Arc::new(DOCAMmap::new().unwrap());
    unsafe {
        Arc::get_mut_unchecked(&mut doca_mmap).add_device(&device).unwrap();
    }

    let dma = DMAEngine::new().unwrap();
    let ctx = DOCAContext::new(&dma, vec![device.clone()]).unwrap();
    /* work queue depth = batch_size */
    let workq = DOCAWorkQueue::new(args.batch_size.try_into().unwrap(), &ctx).unwrap();

    let remote_mmap = Arc::new(
        DOCAMmap::new_from_export(remote_config.export_desc, &device).unwrap()
    );

    /* register remote doca buffer to the inventory */
    let inv = BufferInventory::new(1024).unwrap();
    let remote_dma_buf = DOCARegisteredMemory::new_from_remote(
        &remote_mmap,
        remote_config.remote_addr
    )
        .unwrap()
        .to_buffer(&inv)
        .unwrap();

    /* register local doca buffer to the inventory */
    let local_dma_buf = DOCARegisteredMemory::new(&doca_mmap, local_region)
        .unwrap()
        .to_buffer(&inv)
        .unwrap();

    post_dma_reqs(thread_id, runner.clone(), stat.clone(), args, Arc::new(workq), local_dma_buf, remote_dma_buf);
}