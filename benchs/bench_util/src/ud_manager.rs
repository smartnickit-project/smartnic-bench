use tokio::time::timeout;
use tokio::net::{ TcpListener };
use tokio::io::{ AsyncReadExt, AsyncWriteExt };

use std::time::Duration;
use std::thread::JoinHandle;
use std::{ thread, io };
use std::sync::{ Arc, RwLock };
use std::net::{ SocketAddr };

use std::collections::HashMap;

use crate::ud_endpoint::*;
use crate::MAX_MSG_SZ;
use log::*;

pub struct UdManager {
    pub listen_addr: SocketAddr,
    pub conn_meta: Arc<RwLock<HashMap<u32, Vec<UdMeta>>>>,
    metas_msg: Vec<u8>,
    running: *mut bool,
}

impl UdManager {
    pub fn new(
        listen_addr: SocketAddr,
        conn_meta: Arc<RwLock<HashMap<u32, Vec<UdMeta>>>>,
        metas_msg: Vec<u8>
    ) -> Arc<Self> {
        let running = Box::into_raw(Box::new(true));
        Arc::new(Self {
            listen_addr: listen_addr,
            conn_meta: conn_meta,
            metas_msg: metas_msg,
            running: running,
        })
    }
}

unsafe impl Send for UdManager {}
unsafe impl Sync for UdManager {}

impl UdManager {
    pub fn spawn_server_listener(self: &Arc<Self>) -> JoinHandle<io::Result<()>> {
        let running_addr: u64 = self.running as u64;
        let listener = self.clone();
        thread::spawn(move || {
            tokio::runtime::Builder
                ::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(listener.listen_inner(running_addr))
        })
    }

    async fn listen_inner (self: &Arc<Self>, running_ptr: u64) -> io::Result<()> {
        let mut meta_buff = [0; MAX_MSG_SZ as usize];
        let listener = TcpListener::bind(self.listen_addr).await?;
        // background thread for handshake
        while unsafe { *(running_ptr as *mut bool) } {
            if let Ok(res) = timeout(Duration::from_secs(1), listener.accept()).await {
                let (mut socket, _) = res?;
                let byte_recv = socket.read(&mut meta_buff).await?;
                // println!("Recv a {}-byte message.", byte_recv);
                match byte_recv {
                    0 => {
                        // TCP connection is shutdown
                        unsafe {
                            *(running_ptr as *mut bool) = false;
                        }
                        break;
                    }
                    _ => {
                        info!("Recv a {}-byte connection message.", byte_recv);
                    }
                }
                let (client_meta, client_id) = unmarshal_batch(&meta_buff[0..byte_recv]);
                let old_v = self.conn_meta.write().unwrap().insert(client_id, client_meta);
                if old_v.is_some() {
                    panic!("Wrong in your bootstraping or programming: duplicated connection, client_id: {}", client_id);
                }
                let byte_send = socket.write(self.metas_msg.as_slice()).await?;
                assert!(byte_send != 0);
            }
        }
        Ok(())
    }

    pub fn stop_listen(self : &Arc<Self>) {
        let running_addr: u64 = self.running as u64;
        unsafe { *(running_addr as *mut bool) = false; } 
    }
}