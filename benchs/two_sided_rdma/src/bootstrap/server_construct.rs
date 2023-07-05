use std::sync::{ Arc, RwLock };
use std::borrow::Borrow;
use std::collections::HashMap;
use bench_util::*;
use bench_util::args::*;
use bench_util::doorbell::{ UdDoorbellHelper, RecvDoorbellHelper };
use bench_util::ud_endpoint::*;
use bench_util::ud_message::*;

use netbencher_core::*;

use KRdmaKit::rdma_shim::bindings::*;
use KRdmaKit::{ MemoryRegion, QueuePair, DatagramEndpoint };

use log::*;

pub fn perform_server_routine<T>(
    runner: Arc<BenchRunner<T>>,
    qp: Arc<QueuePair>,
    conn_meta: Arc<RwLock<HashMap<u32, Vec<UdMeta>>>>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let ctx = qp.ctx();
    let mut ud_buffer = UdBuffer::new(MAX_FLYING_MSG, MAX_MSG_SZ);
    let region_size = ud_buffer.get_region_size();
    let (send_mr, recv_mr) = if args.huge_page {
        (
            MemoryRegion::new_huge_page(ctx.clone(), region_size as _).expect(
                "Failed to allocate hugepage MR for send buffer"
            ),
            MemoryRegion::new_huge_page(ctx.clone(), region_size as _).expect(
                "Failed to allocate hugepage MR for send buffer"
            ),
        )
    } else {
        (
            MemoryRegion::new(ctx.clone(), region_size as _).expect(
                "Failed to allocate MR for send buffer"
            ),
            MemoryRegion::new(ctx.clone(), region_size as _).expect(
                "Fail to allocate MR for recv buffer"
            ),
        )
    };

    let mut recv_doorbell = RecvDoorbellHelper::create(MAX_RECV_NUM, qp.clone());
    for wr_id in 0..MAX_FLYING_MSG {
        let start = ud_buffer.get_start_addr();
        recv_doorbell
            .post_recv(&recv_mr, start..start + MAX_MSG_SZ, wr_id)
            .expect("recv should succ");
    }

    let mut completions = [Default::default(); MAX_FLYING_MSG as usize];
    // cache each client-thread's qp endpoint message to avoid fetch read lock every time
    let mut endpoint_cache = HashMap::<u32, Arc<DatagramEndpoint>>::new();
    let batch_or_not = if args.latency_test { 1 } else { args.signal_size };
    let mut pending = 0; // pending unsignaled send requests

    /* payload for reply */
    let payload = align_to_cacheline(0);
    // each loop will recv cqs and post replies
    while runner.running() {
        let recv_comps = qp.poll_recv_cq(&mut completions).unwrap();
        for wc in recv_comps {
            let signal = pending == 0;
            let wr_id = wc.wr_id;

            #[cfg(feature = "OFED_5_4")]
            let ep_id = unsafe { *wc.__bindgen_anon_1.imm_data.as_ref() };

            #[cfg(not(feature = "OFED_5_4"))]
            let ep_id = wc.imm_data;

            let (client_id, client_tid) = decode_id(ep_id);
            let endpoint = match endpoint_cache.get(&ep_id) {
                None => {
                    let client_meta = conn_meta
                        .read()
                        .unwrap()
                        .get(&client_id)
                        .unwrap()
                        .get(client_tid as usize)
                        .unwrap()
                        .clone();
                    // create the cache entry
                    let new_endpoint = Arc::new(
                        DatagramEndpoint::new(
                            qp.ctx(),
                            1,
                            client_meta.lid,
                            client_meta.gid,
                            client_meta.qpn,
                            client_meta.qkey
                        ).unwrap()
                    );
                    endpoint_cache.insert(ep_id, new_endpoint);
                    endpoint_cache.get(&ep_id).unwrap().borrow()
                }
                Some(old_endpoint) => { old_endpoint.borrow() }
            };
            let start = ud_buffer.get_start_addr();
            
            /* reply to client */
            qp.post_datagram(endpoint, &send_mr, start..start + payload, wr_id, signal).expect(
                "send should succeed"
            );
            pending += 1;
            recv_doorbell
                .post_recv(&recv_mr, start..start + MAX_MSG_SZ, wr_id)
                .expect("recv should succ");

            if pending >= batch_or_not {
                let mut ok = false;
                let mut completions = [Default::default()];
                while !ok {
                    let ret = qp.poll_send_cq(&mut completions).expect("Failed to poll send cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            panic!("cq status: {}", ret[0].status);
                        }
                        ok = true;
                    } else {
                    }
                }
                pending = 0;
            }
        }
    }
}

pub fn perform_server_doorbell_routine<T>(
    runner: Arc<BenchRunner<T>>,
    qp: Arc<QueuePair>,
    conn_meta: Arc<RwLock<HashMap<u32, Vec<UdMeta>>>>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let ctx = qp.ctx();
    let mut ud_buffer = UdBuffer::new(MAX_FLYING_MSG, MAX_MSG_SZ);
    let region_size = ud_buffer.get_region_size();
    let (send_mr, recv_mr) = if args.huge_page {
        (
            MemoryRegion::new_huge_page(ctx.clone(), region_size as _).expect(
                "Failed to allocate hugepage MR for send buffer"
            ),
            MemoryRegion::new_huge_page(ctx.clone(), region_size as _).expect(
                "Failed to allocate hugepage MR for send buffer"
            ),
        )
    } else {
        (
            MemoryRegion::new(ctx.clone(), region_size as _).expect(
                "Failed to allocate MR for send buffer"
            ),
            MemoryRegion::new(ctx.clone(), region_size as _).expect(
                "Fail to allocate MR for recv buffer"
            ),
        )
    };

    let mut ud_doorbell = UdDoorbellHelper::create(
        args.db_size,
        ibv_wr_opcode::IBV_WR_SEND,
        qp.clone()
    );
    let mut recv_doorbell = RecvDoorbellHelper::create(MAX_RECV_NUM, qp.clone());
    for wr_id in 0..MAX_FLYING_MSG {
        let start = ud_buffer.get_start_addr();
        recv_doorbell
            .post_recv(&recv_mr, start..start + MAX_MSG_SZ, wr_id)
            .expect("recv should succ");
    }

    let mut completions = [Default::default(); MAX_FLYING_MSG as usize];
    // cache each client-thread's qp endpoint message to avoid fetch read lock every time
    let mut endpoint_cache = HashMap::<u32, Arc<DatagramEndpoint>>::new();
    let batch_or_not = if args.latency_test { 1 } else { args.signal_size };
    let mut pending = 0; // pending unsignaled send requests

    /* payload for reply */
    let payload = align_to_cacheline(0);
    // each loop will recv cqs and post replies w/ doorbell
    while runner.running() {
        let recv_comps = qp.poll_recv_cq(&mut completions).unwrap();
        for wc in recv_comps {
            let signal = pending == 0;
            let wr_id = wc.wr_id;

            #[cfg(feature = "OFED_5_4")]
            let ep_id = unsafe { *wc.__bindgen_anon_1.imm_data.as_ref() };

            #[cfg(not(feature = "OFED_5_4"))]
            let ep_id = wc.imm_data;

            let (client_id, client_tid) = decode_id(ep_id);
            let endpoint = match endpoint_cache.get(&ep_id) {
                None => {
                    let client_meta = conn_meta
                        .read()
                        .unwrap()
                        .get(&client_id)
                        .unwrap()
                        .get(client_tid as usize)
                        .unwrap()
                        .clone();
                    // create the cache entry
                    let new_endpoint = Arc::new(
                        DatagramEndpoint::new(
                            qp.ctx(),
                            1,
                            client_meta.lid,
                            client_meta.gid,
                            client_meta.qpn,
                            client_meta.qkey
                        ).unwrap()
                    );
                    endpoint_cache.insert(ep_id, new_endpoint);
                    endpoint_cache.get(&ep_id).unwrap().borrow()
                }
                Some(old_endpoint) => { old_endpoint.borrow() }
            };
            let start = ud_buffer.get_start_addr();
            ud_doorbell
                .post_send(endpoint, &send_mr, start..start + payload, wr_id, None, signal)
                .expect("send should succeed");
            pending += 1;
            recv_doorbell
                .post_recv(&recv_mr, start..start + MAX_MSG_SZ, wr_id)
                .expect("recv should succ");

            if pending >= batch_or_not {
                let mut ok = false;
                let mut completions = [Default::default()];
                while !ok {
                    let ret = qp.poll_send_cq(&mut completions).expect("Failed to poll send cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            error!("cq status: {}", ret[0].status);
                        }
                        ok = true;
                    }
                }
                pending = 0;
            }
        }
    }
}