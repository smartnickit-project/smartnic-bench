use doca::{ RawPointer, RawPointerMsg, DOCAMmap };
use doca::open_device_with_pci;
use serde_derive::{ Serialize, Deserialize };

pub const DOCA_MAX_CONN_LENGTH: usize = 4096;

#[derive(Serialize, Deserialize)]
pub struct DocaConnInfoMsg {
    pub exports: Vec<Vec<u8>>,
    pub buffers: Vec<RawPointerMsg>,
}

#[derive(Clone)]
pub struct DocaConnInfo {
    pub exports: Vec<Vec<u8>>,
    pub buffers: Vec<RawPointer>,
}

impl Default for DocaConnInfo {
    fn default() -> Self {
        Self {
            exports: Vec::new(),
            buffers: Vec::new(),
        }
    }
}

impl From<DocaConnInfo> for DocaConnInfoMsg {
    fn from(info: DocaConnInfo) -> Self {
        Self {
            exports: info.exports,
            buffers: info.buffers.into_iter().map(|v| v.into()).collect(),
        }
    }
}

impl From<DocaConnInfoMsg> for DocaConnInfo {
    fn from(msg: DocaConnInfoMsg) -> Self {
        Self {
            exports: msg.exports,
            buffers: msg.buffers.into_iter().map(|v| v.into()).collect(),
        }
    }
}

impl DocaConnInfo {
    pub fn serialize(data: DocaConnInfo) -> Vec<u8> {
        let msg: DocaConnInfoMsg = data.into();
        serde_json::to_vec(&msg).unwrap()
    }

    pub fn deserialize(data: &[u8]) -> DocaConnInfo {
        let msg: DocaConnInfoMsg = serde_json::from_slice(data).unwrap();
        let data: DocaConnInfo = msg.into();
        data
    }
}