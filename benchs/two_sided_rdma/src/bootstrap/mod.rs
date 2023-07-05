mod client_construct;
pub use client_construct::{ 
    perform_client_routine, 
    perform_client_doorbell_routine,
    perform_client_profile_routine,
};

mod server_construct;
pub use server_construct::{ perform_server_routine, perform_server_doorbell_routine };

use std::{ thread, time };
use std::net::{ SocketAddr, TcpStream };
use std::time::Duration;
use std::collections::HashMap;
use std::sync::{ Arc, RwLock };

use tokio::runtime::Runtime;

use bench_util::args::*;
use bench_util::ud_endpoint::*;
use bench_util::*;
use bench_util::ud_manager::*;

use netbencher_core::{
    CoordinatedReporterMaster,
    BenchRunner,
    SimpleBenchReporter,
    CoordinatedReporter,
};

use log::*;

pub fn bootstrap_client(args: CmdlineArgs) {
    let listen_addr: SocketAddr = args.listen_addr.parse().unwrap();
    let mut socket = TcpStream::connect(listen_addr).unwrap();

    // create and connect all UD qps for all threads of client
    let (client_qps, server_eps) = bootstrap_uds(
        &mut socket,
        args.nic_idx,
        args.nic_num,
        args.threads as usize,
        args.client_id
    );

    let mut runner = BenchRunner::new(args.threads.try_into().unwrap());
    runner.run(move |thread_id, runner, stat, args| {
        match (args.profile, args.doorbell) {
            (false, false) => {
                perform_client_routine(
                    thread_id, 
                    runner, 
                    stat,
                    client_qps[thread_id].clone(),
                    server_eps[thread_id].clone(), 
                    args
                );
            }
            (false, true) => {
                info!("features: doorbell");
                perform_client_doorbell_routine(
                    thread_id, 
                    runner, 
                    stat, 
                    client_qps[thread_id].clone(),
                    server_eps[thread_id].clone(),
                    args
                );
            }
            (true, false) => {
                info!("features: profile");
                perform_client_profile_routine(
                    thread_id, 
                    runner, 
                    stat, 
                    client_qps[thread_id].clone(),
                    server_eps[thread_id].clone(),
                    args
                );
            }
            (true, true) => {
                warn!("We dont support profiling doorbell send for now!");
            }
        }
    }, args.clone());

    let mut inner_reporter = SimpleBenchReporter::new_with_id(args.client_id.try_into().unwrap());

    if args.report {
        Runtime::new()
            .unwrap()
            .block_on(async {
                let mut reporter = CoordinatedReporter::new(
                    args.report_addr.parse().unwrap(),
                    inner_reporter
                ).await.expect("failed to create the reporter");

                // send a report to the master
                for epoch in 0..args.life {
                    thread::sleep(time::Duration::from_secs(1));
                    runner.report_async(&mut reporter).await;
                }
            });
    } else {
        for epoch in 0..args.life {
            thread::sleep(time::Duration::from_secs(1));
            info!("{}", runner.report(&mut inner_reporter));
        }
    }
    runner.stop().unwrap();
}

pub fn bootstrap_server(mut args: CmdlineArgs) {
    if args.life < MIN_SERVER_LIFE {
        args.life = MIN_SERVER_LIFE;
    }

    let conn_meta: Arc<RwLock<HashMap<u32, Vec<UdMeta>>>> = Default::default();
    let listen_addr: SocketAddr = args.listen_addr.parse().unwrap();
    // Create UD qps that are automately assigned to each NIC port
    // After bootstraping, server is ready to be connected
    let (qps, metas) = bootstrap_ud_server(args.threads as usize, args.nic_idx, args.nic_num);
    
    // create a copy for closure
    let conn_meta_ptr = conn_meta.clone();
    let mut runner = BenchRunner::new(args.threads.try_into().unwrap());
    runner.run(move |thread_id, runner, stat, args| {
        match args.doorbell {
            false => {
                perform_server_routine(runner, qps[thread_id].clone(), conn_meta_ptr.clone(), args);
            }
            true => {
                info!("features: doorbell");
                perform_server_doorbell_routine(runner, qps[thread_id].clone(), conn_meta_ptr.clone(), args);
            }
        }
    }, args.clone());

    // serialize meta infos of server's UD qps into message
    let metas_msg = marshal_batch(metas, 0);
    // wait for each client's connect message and the final TERMINATE_SIG
    let ud_manager = UdManager::new(listen_addr, conn_meta, metas_msg);
    let listen_thread = ud_manager.spawn_server_listener();

    // start to collecting reports
    if args.report {
        Runtime::new()
            .unwrap()
            .block_on(async {
                let mut master = CoordinatedReporterMaster::new(
                    MAX_CLIENTS,
                    args.report_addr.parse().unwrap()
                ).await.expect("failed to create the master");

                master
                    .report_event_loop(
                        Duration::from_secs(args.life.into()),
                        Duration::from_secs(1)
                    ).await
                    .expect("event loop report error");
            });
    } else {
        thread::sleep(Duration::from_secs(args.life.into()));
    }

    // stop listening and exit
    ud_manager.stop_listen();
    listen_thread.join();
    runner.stop().unwrap();
    info!("Server exit.");
}