use std::sync::{ Arc };
use std::borrow::Borrow;
use bench_util::*;
use bench_util::args::*;
use bench_util::doorbell::{ UdDoorbellHelper, RecvDoorbellHelper };
use bench_util::ud_message::*;
use bench_util::ud_endpoint::*;

#[cfg(not(feature = "ARM"))]
use bench_util::rdtsc::*;

use netbencher_core::*;

use KRdmaKit::rdma_shim::bindings::*;
use KRdmaKit::{ QueuePair, DatagramEndpoint, MemoryRegion };

use log::*;

pub fn perform_client_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    client_qp: Arc<QueuePair>,
    server_ep: Arc<DatagramEndpoint>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let ctx = client_qp.ctx();
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

    let mut recv_doorbell = RecvDoorbellHelper::create(MAX_RECV_NUM, client_qp.clone());
    for wr_id in 0..MAX_FLYING_MSG {
        let start = ud_buffer.get_start_addr();
        recv_doorbell
            .post_recv(&recv_mr, start..start + MAX_MSG_SZ, wr_id)
            .expect("recv should succ");
    }

    let mut completions = [Default::default(); MAX_FLYING_MSG as usize];
    let mut pending: usize = 0;
    let batch_or_not = if args.latency_test { 1 } else { args.signal_size };
    // encode client message to imm so that server can know who to reply to
    let imm_data = encode_id(args.client_id as _, thread_id as _);
    let payload = align_to_cacheline(args.payload);
    // each loop send args.factor UD msgs and wait for their replies
    while runner.running() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
        let req_batch = if args.latency_test { 1 } else { args.factor };
        for i in 0..req_batch {
            let signal = pending == 0;
            let start = ud_buffer.get_start_addr();

            client_qp
                .post_datagram_w_imm(
                    server_ep.borrow(),
                    &send_mr,
                    start..start + payload,
                    i,
                    imm_data,
                    signal
                )
                .expect("send should succeeed");
            pending += 1;
            recv_doorbell
                .post_recv(&recv_mr, start..start + MAX_MSG_SZ, i)
                .expect("recv should succ");
            if pending >= batch_or_not {
                let mut ok = false;
                let mut completions = [Default::default()];
                while !ok {
                    let ret = client_qp.poll_send_cq(&mut completions).expect("Failed to poll cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            panic!("cq status: {}", ret[0].status);
                        }
                        ok = true;
                    }
                }
                pending = 0;
            }
        }

        let mut remaining = req_batch as i64;
        while remaining > 0 && runner.running() {
            let recv = client_qp.poll_recv_cq(&mut completions).unwrap();
            let recv_msg_num = recv.len() as u64;
            remaining -= recv_msg_num as i64;
            if remaining < 0 {
                panic!(
                    "Wrong in your programming, reply to an false client. Num of additional message: {}",
                    -remaining
                );
            }
            unsafe {
                Arc::get_mut_unchecked(&mut stat).finished_batch_ops(recv_msg_num);
            }
        }
    }
}

pub fn perform_client_profile_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    client_qp: Arc<QueuePair>,
    server_ep: Arc<DatagramEndpoint>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy 
    
{
    let ctx = client_qp.ctx();
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

    let mut recv_doorbell = RecvDoorbellHelper::create(MAX_RECV_NUM, client_qp.clone());
    for wr_id in 0..MAX_FLYING_MSG {
        let start = ud_buffer.get_start_addr();
        recv_doorbell
            .post_recv(&recv_mr, start..start + MAX_MSG_SZ, wr_id)
            .expect("recv should succ");
    }

    let mut completions = [Default::default(); MAX_FLYING_MSG as usize];
    let mut pending: usize = 0;
    let batch_or_not = if args.latency_test { 1 } else { args.signal_size };
    // encode client message to imm so that server can know who to reply to
    let imm_data = encode_id(args.client_id as _, thread_id as _);
    let payload = align_to_cacheline(args.payload);
    // each loop send args.factor UD msgs and wait for their replies
    while runner.running() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
        let req_batch = if args.latency_test { 1 } else { args.factor };
        for i in 0..req_batch {
            let signal = pending == 0;
            let start = ud_buffer.get_start_addr();
            #[cfg(not(feature = "ARM"))] 
            let begin_ts = get_rdtsc();

            client_qp
                .post_datagram_w_imm(
                    server_ep.borrow(),
                    &send_mr,
                    start..start + payload,
                    i,
                    imm_data,
                    signal
                )
                .expect("send should succeeed");
            #[cfg(not(feature = "ARM"))]
            {
                let end_ts = get_rdtsc();
                unsafe {
                    Arc::get_mut_unchecked(&mut stat).record_avg_rdtsc((end_ts - begin_ts).try_into().unwrap());
                }
            }   
            
            pending += 1;
            recv_doorbell
                .post_recv(&recv_mr, start..start + MAX_MSG_SZ, i)
                .expect("recv should succ");
            if pending >= batch_or_not {
                let mut ok = false;
                let mut completions = [Default::default()];
                while !ok {
                    let ret = client_qp.poll_send_cq(&mut completions).expect("Failed to poll cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            panic!("cq status: {}", ret[0].status);
                        }
                        ok = true;
                    }
                }
                pending = 0;
            }
        }

        let mut remaining = req_batch as i64;
        while remaining > 0 && runner.running() {
            let recv = client_qp.poll_recv_cq(&mut completions).unwrap();
            let recv_msg_num = recv.len() as u64;
            remaining -= recv_msg_num as i64;
            if remaining < 0 {
                panic!(
                    "Wrong in your programming, reply to an false client. Num of additional message: {}",
                    -remaining
                );
            }
            unsafe {
                Arc::get_mut_unchecked(&mut stat).finished_batch_ops(recv_msg_num);
            }
        }
    }
}

pub fn perform_client_doorbell_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    client_qp: Arc<QueuePair>,
    server_ep: Arc<DatagramEndpoint>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy 
{
    let ctx = client_qp.ctx();
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
        ibv_wr_opcode::IBV_WR_SEND_WITH_IMM,
        client_qp.clone()
    );
    let mut recv_doorbell = RecvDoorbellHelper::create(MAX_RECV_NUM, client_qp.clone());
    for wr_id in 0..MAX_FLYING_MSG {
        let start = ud_buffer.get_start_addr();
        recv_doorbell
            .post_recv(&recv_mr, start..start + MAX_MSG_SZ, wr_id)
            .expect("recv should succ");
    }

    let mut completions = [Default::default(); MAX_FLYING_MSG as usize];
    let mut pending: usize = 0;
    let batch_or_not = if args.latency_test { 1 } else { args.signal_size };
    // encode client message to imm so that server can know who to reply to
    let imm_data = encode_id(args.client_id as _, thread_id as _);
    let payload = align_to_cacheline(args.payload);
    // each loop send args.factor UD msgs and wait for their replies
    while runner.running() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
        let req_batch = if args.latency_test { 1 } else { args.factor };
        for i in 0..req_batch {
            let signal = pending == 0;
            let start = ud_buffer.get_start_addr();

            ud_doorbell
                .post_send(
                    server_ep.borrow(),
                    &send_mr,
                    start..start + payload,
                    i,
                    Some(imm_data),
                    signal
                )
                .expect("send should succeeed");
            pending += 1;
            recv_doorbell
                .post_recv(&recv_mr, start..start + MAX_MSG_SZ, i)
                .expect("recv should succ");
            if pending >= batch_or_not {
                let mut ok = false;
                let mut completions = [Default::default()];
                while !ok {
                    let ret = client_qp.poll_send_cq(&mut completions).expect("Failed to poll cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            panic!("cq status: {}", ret[0].status);
                        }
                        ok = true;
                    }
                }
                pending = 0;
            }
        }

        let mut remaining = req_batch as i64;
        while remaining > 0 && runner.running() {
            let recv = client_qp.poll_recv_cq(&mut completions).unwrap();
            let recv_msg_num = recv.len() as u64;
            remaining -= recv_msg_num as i64;
            if remaining < 0 {
                panic!(
                    "Wrong in your programming, reply to an false client. Num of additional message: {}",
                    -remaining
                );
            }
            unsafe {
                Arc::get_mut_unchecked(&mut stat).finished_batch_ops(recv_msg_num);
            }
        }
    }
}