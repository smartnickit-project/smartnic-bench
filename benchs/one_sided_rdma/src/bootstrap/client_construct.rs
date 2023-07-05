use std::sync::{ Arc };
use bench_util::args::*;
use bench_util::doorbell::RcDoorbellHelper;

use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha8Rng;

use netbencher_core::*;

use KRdmaKit::rdma_shim::bindings::*;

use log::*;

pub fn perform_client_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let batch_or_not = if args.latency_test { 1 } else { args.signal_size };
    let (qp, client_mr, server_meta) = match args.create_rc(thread_id) {
        Ok(res) => res,
        Err(()) => { panic!("Fail to bring up RC qp!") }
    };
    let mut rand = ChaCha8Rng::seed_from_u64(
        ((0xdeadbeaf + 73 * thread_id) as u64) + args.client_id * 37
    );
    let mut completions = [Default::default()];

    let mut pending: usize = 0;
    let start = 0;

    while runner.running() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
        let mut start = 0;
        for i in 0..args.factor {
            let index = args.get_next_index(thread_id, &mut rand);
            let signal = pending == 0;
            if args.read {
                qp.post_send_read(
                    &client_mr,
                    start..start + args.payload,
                    signal,
                    server_meta.addr + index,
                    server_meta.rkey,
                    i
                ).expect("read should succeeed");
            } else {
                qp.post_send_write(
                    &client_mr,
                    start..start + args.payload,
                    signal,
                    server_meta.addr + index,
                    server_meta.rkey,
                    i
                ).expect("read should succeeed");
            }
            start += args.payload;
            pending += 1;
            if pending >= batch_or_not {
                let mut ok = false;
                while !ok {
                    let ret = qp.poll_send_cq(&mut completions).expect("Failed to poll cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            error!("read remote addr: {:?} err: {}", index, ret[0].status);
                        }
                        assert_eq!(ret[0].status, 0);
                        ok = true;
                    }
                }
                pending = 0;
            }
        }
        unsafe {
            Arc::get_mut_unchecked(&mut stat).finished_batch_ops(args.factor);
        }
    } // end of main benchmark loop
}

pub fn perform_client_doorbell_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let batch_or_not = if args.latency_test { 1 } else { args.signal_size };
    let (qp, client_mr, server_meta) = match args.create_rc(thread_id) {
        Ok(res) => res,
        Err(()) => { panic!("Fail to bring up RC qp!") }
    };
    let mut rand = ChaCha8Rng::seed_from_u64(
        ((0xdeadbeaf + 73 * thread_id) as u64) + args.client_id * 37
    );

    let mut completions = [Default::default()];
    let mut pending: usize = 0;
    let mut rc_doorbell = RcDoorbellHelper::create(args.db_size, qp.clone());
    if args.read {
        rc_doorbell.init(ibv_wr_opcode::IBV_WR_RDMA_READ);
    } else {
        rc_doorbell.init(ibv_wr_opcode::IBV_WR_RDMA_WRITE);
    }

    while runner.running() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
        let mut start = 0;
        for i in 0..args.factor {
            let index = args.get_next_index(thread_id, &mut rand);
            // start = (start + std::cmp::max(PAYLOAD, 64)) % ((LOCAL_MR - PAYLOAD) as u64);
            let signal = pending == 0;

            rc_doorbell
                .post_send(
                    &client_mr,
                    start..start + args.payload,
                    signal,
                    server_meta.addr + index,
                    server_meta.rkey,
                    i
                )
                .expect("read should succeeed");
                
            start += args.payload;
            pending += 1;
            if pending >= batch_or_not {
                let mut ok = false;
                while !ok {
                    let ret = qp.poll_send_cq(&mut completions).expect("Failed to poll cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            error!("read remote addr: {:?} err", index);
                        }
                        assert_eq!(ret[0].status, 0);
                        ok = true;
                    }
                }
                pending = 0;
            }
        }
        unsafe {
            Arc::get_mut_unchecked(&mut stat).finished_batch_ops(args.factor);
        }
    }
}

pub fn perform_client_signaled_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let (qp, client_mr, server_meta) = match args.create_rc(thread_id) {
        Ok(res) => res,
        Err(()) => { panic!("Fail to bring up RC qp!") }
    };
    let mut rand = ChaCha8Rng::seed_from_u64(
        ((0xdeadbeaf + 73 * thread_id) as u64) + args.client_id * 37
    );
    let mut completions = [Default::default()];
    
    while runner.running() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
        let mut start = 0;
        for i in 0..args.factor {
            let index = args.get_next_index(thread_id, &mut rand);
            // start = (start + std::cmp::max(PAYLOAD, 64)) % ((LOCAL_MR - PAYLOAD) as u64);
            if args.read {
                qp.post_send_read(
                    &client_mr,
                    start..start + args.payload,
                    true,
                    server_meta.addr + index,
                    server_meta.rkey,
                    i
                ).expect("read should succeeed");
            } else {
                qp.post_send_write(
                    &client_mr,
                    start..start + args.payload,
                    true,
                    server_meta.addr + index,
                    server_meta.rkey,
                    i
                ).expect("read should succeeed");
            }

            start += args.payload;
        }

        for _ in 0..args.factor {
            let mut ok = false;
            while !ok {
                let ret = qp.poll_send_cq(&mut completions).expect("Failed to poll cq");
                if ret.len() > 0 {
                    if ret[0].status != 0 {
                    }
                    assert_eq!(ret[0].status, 0);
                    ok = true;
                }
            }
        }
        unsafe {
            Arc::get_mut_unchecked(&mut stat).finished_batch_ops(args.factor);
        }
    } // end of main benchmark loop
}

pub fn perform_client_doorbell_signaled_routine<T>(
    thread_id: usize,
    runner: Arc<BenchRunner<T>>,
    mut stat: Arc<BenchStat>,
    args: CmdlineArgs
)
    where T: Send + 'static + Sync + Copy
{
    let batch_or_not = 1;
    let (qp, client_mr, server_meta) = match args.create_rc(thread_id) {
        Ok(res) => res,
        Err(()) => { panic!("Fail to bring up RC qp!") }
    };
    let mut rand = ChaCha8Rng::seed_from_u64(
        ((0xdeadbeaf + 73 * thread_id) as u64) + args.client_id * 37
    );

    let mut completions = [Default::default()];
    let mut pending: usize = 0;
    let mut rc_doorbell = RcDoorbellHelper::create(args.db_size, qp.clone());
    if args.read {
        rc_doorbell.init(ibv_wr_opcode::IBV_WR_RDMA_READ);
    } else {
        rc_doorbell.init(ibv_wr_opcode::IBV_WR_RDMA_WRITE);
    }

    while runner.running() {
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::Release);
        let mut start = 0;
        for i in 0..args.factor {
            let index = args.get_next_index(thread_id, &mut rand);
            // start = (start + std::cmp::max(PAYLOAD, 64)) % ((LOCAL_MR - PAYLOAD) as u64);
            let signal = pending == 0;

            rc_doorbell
                .post_send(
                    &client_mr,
                    start..start + args.payload,
                    signal,
                    server_meta.addr + index,
                    server_meta.rkey,
                    i
                )
                .expect("read should succeeed");
            start += args.payload;
            pending += 1;
            if pending >= batch_or_not {
                let mut ok = false;
                while !ok {
                    let ret = qp.poll_send_cq(&mut completions).expect("Failed to poll cq");
                    if ret.len() > 0 {
                        if ret[0].status != 0 {
                            error!("read remote addr: {:?} err", index);
                        }
                        assert_eq!(ret[0].status, 0);
                        ok = true;
                    }
                }
                pending = 0;
            }
        }
        unsafe {
            Arc::get_mut_unchecked(&mut stat).finished_batch_ops(args.factor);
        }
    }
}