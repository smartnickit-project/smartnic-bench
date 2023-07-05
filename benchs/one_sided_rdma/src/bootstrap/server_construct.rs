use std::{ time, thread };
use std::net::{ SocketAddr };
use std::sync::{ Arc }; 
use std::sync::atomic::{ compiler_fence, Ordering };

use bench_util::args::CmdlineArgs;

use KRdmaKit::{ UDriver, MemoryRegion };
use KRdmaKit::services_user::{ ConnectionManagerServer, DefaultConnectionManagerHandler };

use netbencher_core::BenchRunner;

use log::*;

pub fn perform_server_routine<T>(runner: Arc<BenchRunner<T>>, args: CmdlineArgs)
    where T: Send + 'static + Sync + Copy
{
    debug!("server uses RNIC {}", args.nic_idx);

    // bootstrap one-sided RDMA server
    let ctx = UDriver::create()
        .expect("failed to query device")
        .devices()
        .get(args.nic_idx)
        .expect("no rdma device available")
        .open_context()
        .expect("failed to create RDMA context");

    info!("Check registered huge page sz: {}KB", args.random_space / 1024);

    let mut handler = DefaultConnectionManagerHandler::new(&ctx, 1);
    let server_mr = if args.huge_page {
        MemoryRegion::new_huge_page(ctx.clone(), args.random_space as usize).expect(
            "Failed to allocate huge page MR"
        )
    } else {
        MemoryRegion::new(ctx.clone(), args.random_space as usize).expect("Failed to allocate MR")
    };

    handler.register_mr(vec![("MR".to_string(), server_mr)]);
    let server = ConnectionManagerServer::new(handler);
    let listen_addr: SocketAddr = args.listen_addr.parse().unwrap();

    /* set listener, the server_thread listens for connection requests */
    let server_thread = server.spawn_listener(listen_addr);

    while runner.running() {
        compiler_fence(Ordering::SeqCst);
    }
    server.stop_listening();
    // wait for listeners to exit
    let _ = server_thread.join();
    info!("Exit");
}