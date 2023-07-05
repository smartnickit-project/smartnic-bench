use serde_derive::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::borrow::Borrow;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use KRdmaKit::{DatagramEndpoint, QueuePair, QueuePairBuilder, UDriver};
use KRdmaKit::rdma_shim::bindings::*;
use log::*;

use crate::MAX_MSG_SZ;


pub const TERMINATE_SIG: usize = 1;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UdMetaRaw {
    pub gid: ibv_gid_wrapper,
    pub lid: u32,
    pub qpn: u32,
    pub qkey: u32,
}

#[derive(Clone)]
pub struct UdMeta {
    pub gid: ib_gid,
    pub lid: u32,
    pub qpn: u32,
    pub qkey: u32,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UdMetaBatch {
    pub a: Vec<UdMetaRaw>,
    pub b: u32,
}

#[repr(C)]
#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct ibv_gid_wrapper {
    raw: [u64; 2usize],
}

impl From<ib_gid> for ibv_gid_wrapper {
    fn from(gid: ib_gid) -> Self {
        Self {
            raw: gid.bindgen_union_field,
        }
    }
}

impl Into<ib_gid> for ibv_gid_wrapper {
    fn into(self) -> ib_gid {
        ib_gid {
            bindgen_union_field: self.raw,
            ..Default::default()
        }
    }
}

pub fn marshal(msg: UdMeta) -> Vec<u8> {
    let data = UdMetaRaw {
        gid: ibv_gid_wrapper::from(msg.gid),
        lid: msg.lid,
        qpn: msg.qpn,
        qkey: msg.qkey,
    };
    serde_json::to_vec(&data).unwrap()
}

pub fn unmarshal(raw: &[u8]) -> UdMeta {
    let msg: UdMetaRaw = serde_json::from_slice(raw).unwrap();
    UdMeta {
        gid: msg.gid.into(),
        lid: msg.lid,
        qpn: msg.qpn,
        qkey: msg.qkey,
    }
}

pub fn marshal_batch(msg: Vec<UdMeta>, msg_id: u32) -> Vec<u8> {
    let data: Vec<UdMetaRaw> = msg
        .into_iter()
        .map(|msg| UdMetaRaw {
            gid: ibv_gid_wrapper::from(msg.gid),
            lid: msg.lid,
            qpn: msg.qpn,
            qkey: msg.qkey,
        })
        .collect();
    let data = UdMetaBatch { a: data, b: msg_id };
    serde_json::to_vec(&data).unwrap()
}

pub fn unmarshal_batch(raw: &[u8]) -> (Vec<UdMeta>, u32) {
    let msg: UdMetaBatch = serde_json::from_slice(raw).unwrap();
    (
        msg.a
            .into_iter()
            .map(|msg| UdMeta {
                gid: msg.gid.into(),
                lid: msg.lid,
                qpn: msg.qpn,
                qkey: msg.qkey,
            })
            .collect(),
        msg.b,
    )
}

pub fn bootstrap_uds(
    socket: &mut TcpStream,
    nic_idx: usize,
    nic_num: usize,
    threads: usize,
    client_id: u64,
) -> (Vec<Arc<QueuePair>>, Vec<Arc<DatagramEndpoint>>) {
    let mut client_qps = Vec::new();
    let mut client_raw_eps = Vec::new();
    let mut server_datagram_eps = Vec::new();

    // create one ud qp for each thread
    for tid in 0..threads {
        let ctx = UDriver::create()
            .expect("failed to query device")
            .devices()
            .get(nic_idx + (tid % nic_num))
            .expect("no rdma device available")
            .open_context()
            .expect("failed to create RDMA context");
        let client_qp = QueuePairBuilder::new(&ctx)
            .build_ud()
            .expect("fail to build ud qp")
            .bring_up_ud()
            .expect("fail to bring up ud qp");
        let client_ep = UdMeta {
            gid: client_qp.gid().unwrap(),
            lid: client_qp.lid().unwrap(),
            qpn: client_qp.qp_num(),
            qkey: client_qp.qkey(),
        };
        client_qps.push(client_qp);
        client_raw_eps.push(client_ep);
    }

    // exchange endpoints message to build ud connection
    let client_raw_eps_msg = marshal_batch(client_raw_eps, client_id as u32);
    let mut msg_buf = [0; MAX_MSG_SZ as usize];
    let byte_send = socket.write(client_raw_eps_msg.as_slice()).unwrap();
    let byte_recv = socket.read(&mut msg_buf).unwrap();
    debug!("[CONN] send {} bytes, recv {} bytes", byte_send, byte_recv);

    let (server_raw_eps, _) = unmarshal_batch(&msg_buf[0..byte_recv]);
    let srv_threads = server_raw_eps.len();

    /* Create server endpoints for each client thread
    Notice: in case `threads != srv_threads`
    - Create the first `SRV_THREADS` endpoints for each client thread, 
        then create the remaining according to them
    - The mapping between client and server is:
        client_thread % srv_threads => server_thread
    */
    for (id, server_raw_ep) in server_raw_eps.into_iter().enumerate() {
        let ctx = client_qps[id].ctx();
        server_datagram_eps.push(Arc::new(
            DatagramEndpoint::new(
                &ctx,1,
                server_raw_ep.lid,
                server_raw_ep.gid,
                server_raw_ep.qpn,
                server_raw_ep.qkey,
            ).unwrap(),
        ));
    }
    for id in srv_threads..(threads as _) {
        debug!("client {}->server {}", id, id % srv_threads);
        let ctx = client_qps[id].ctx();
        let server_ep: &Arc<DatagramEndpoint> = server_datagram_eps
                                                .get(id % srv_threads).unwrap().borrow();
        server_datagram_eps.push(Arc::new(
            DatagramEndpoint::new(
                &ctx,1,
                server_ep.lid(),
                server_ep.gid(),
                server_ep.qpn(),
                server_ep.qkey(),
            ).unwrap(),
        ));
    }
    (client_qps, server_datagram_eps)
}

pub fn terminate_server(
    socket: &mut TcpStream
) {
    let mut msg: [u8; 1] = [0];
    let byte_send = socket.write(&mut msg).unwrap();
    debug!("Send a {}-byte termination message to server.", byte_send);
    assert!(byte_send == TERMINATE_SIG);
}

pub fn bootstrap_ud_server(
    threads: usize,
    nic_idx: usize,
    nic_num: usize,
) -> (Vec<Arc<QueuePair>>, Vec<UdMeta>) {
    let mut server_qps = Vec::new();
    let mut server_metas = Vec::new();

    for tid in 0..threads {
        let ctx = UDriver::create()
            .expect("failed to query device")
            .devices()
            .get(nic_idx + (tid % nic_num))
            .expect("no rdma device available")
            .open_context()
            .expect("failed to create RDMA context");

        let server_qp = QueuePairBuilder::new(&ctx)
            .build_ud()
            .expect("failed to build UD QP")
            .bring_up_ud()
            .expect("failed to bring up UD QP");
        let server_meta = UdMeta {
            gid: server_qp.gid().unwrap(),
            lid: server_qp.lid().unwrap(),
            qpn: server_qp.qp_num(),
            qkey: server_qp.qkey(),
        };
        server_qps.push(server_qp);
        server_metas.push(server_meta);
    }
    (server_qps, server_metas)
}

pub fn encode_id(
    client_id: u32,
    thread_id: u32
) -> u32 
{
    (client_id << 16) | thread_id
}

pub fn decode_id(
    imm_data: u32
) -> (u32, u32) {
    let client_id: u32 = imm_data >> 16;
    let thread_id: u32 = imm_data & (0xffff);
    (client_id, thread_id)
}