use common::basic_types::{ManagedElement16, ManagedVecMapVec, ManagedVecU16};
use dtp_sdk::{Host, DTP};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct OneShotCallbackMessage {
    pub cid: u64,
    pub response: String,
}

#[derive(Debug)]
pub struct OneshotCallback {
    pub cid: u64,
    pub host_sla_idx: ManagedVecU16,
    pub tc: String,
    pub resp_channel: Option<tokio::sync::oneshot::Sender<OneShotCallbackMessage>>,
    pub block_channel: Option<tokio::sync::oneshot::Receiver<OneShotCallbackMessage>>,
}

#[derive(Debug)]
// One per host_sla_idx (driven by config).
//
// Used for variables operating on the client side. See DTPConnStateDataServer for the server side.
pub struct DTPConnStateDataClient {
    pub idx: Option<ManagedVecU16>,
    pub is_open: bool,
    pub dtp: Option<Arc<Mutex<DTP>>>,
    pub host: Option<Host>, // This is the client side host.
}

impl DTPConnStateDataClient {
    pub fn new() -> Self {
        Self {
            idx: None,
            is_open: false,
            dtp: None,
            host: None,
        }
    }

    pub fn set_dtp(&mut self, dtp: &Arc<Mutex<DTP>>) {
        self.dtp = Some(Arc::clone(dtp));
    }

    pub fn set_host(&mut self, host: Host) {
        self.host = Some(host);
    }
}

impl std::default::Default for DTPConnStateDataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ManagedElement16 for DTPConnStateDataClient {
    fn idx(&self) -> Option<ManagedVecU16> {
        self.idx
    }

    fn set_idx(&mut self, index: Option<ManagedVecU16>) {
        self.idx = index;
    }
}

#[derive(Debug)]
pub struct GlobalsDTPConnsStateClientST {
    pub subs_next_cid: u64,
    pub send_next_cid: u64,

    pub subs_callbacks: HashMap<ManagedVecU16, OneshotCallback>,
    // Key is the TransportProtocol address ("0x" string).
    pub send_callbacks: HashMap<String, OneshotCallback>,
    pub conns: ManagedVecMapVec<DTPConnStateDataClient>,
}

impl GlobalsDTPConnsStateClientST {
    pub fn new() -> Self {
        Self {
            subs_next_cid: 1,
            send_next_cid: 1,
            subs_callbacks: HashMap::new(),
            send_callbacks: HashMap::new(),
            conns: ManagedVecMapVec::new(),
        }
    }

    pub fn create_subs_callback(&mut self, host_sla_idx: ManagedVecU16) -> u64 {
        let cid = self.subs_next_cid;
        self.subs_next_cid += 1;
        // Create a one-shot channel
        let (tx, rx) = tokio::sync::oneshot::channel();
        let callback = OneshotCallback {
            cid,
            host_sla_idx,
            tc: "".to_string(),
            resp_channel: Some(tx),
            block_channel: Some(rx),
        };
        // Add to subs_callbacks.
        self.subs_callbacks.insert(host_sla_idx, callback);
        // Caller will use the cid to block on the channel response.
        cid
    }

    pub fn get_subs_callback(
        &mut self,
        cid: u64,
    ) -> Option<tokio::sync::oneshot::Receiver<OneShotCallbackMessage>> {
        // Iterate the subs_callbacks, find the entry with this cid.
        // Get the block_channel.
        // Block on the block_channel (await).
        // Remove the entry from subs_callbacks.
        for (_, callback) in self.subs_callbacks.iter_mut() {
            if callback.cid == cid {
                return Some(callback.block_channel.take().unwrap());
            }
        }
        None
    }

    pub fn trigger_subs_callback(&mut self, host_sla_idx: ManagedVecU16) {
        if let Some(callback) = self.subs_callbacks.get_mut(&host_sla_idx) {
            if let Some(channel) = callback.resp_channel.take() {
                let msg = OneShotCallbackMessage {
                    cid: callback.cid,
                    response: "".to_string(),
                };
                let result = channel.send(msg);
                if let Err(e) = result {
                    log::error!("Error sending response to callback: {:?}", e);
                } else {
                    log::info!("Triggered callback for host_sla_idx: {:?}", host_sla_idx);
                }
            }
        }
    }

    pub fn delete_subs_callback(&mut self, host_sla_idx: ManagedVecU16) {
        self.subs_callbacks.remove(&host_sla_idx);
    }

    pub fn create_send_callback(&mut self, host_sla_idx: ManagedVecU16, tc: String) -> u64 {
        let cid = self.send_next_cid;
        self.send_next_cid += 1;
        // Create a one-shot channel
        let (tx, rx) = tokio::sync::oneshot::channel();
        let callback = OneshotCallback {
            cid,
            host_sla_idx,
            tc: tc.clone(),
            resp_channel: Some(tx),
            block_channel: Some(rx),
        };
        // Add to subs_callbacks.
        self.send_callbacks.insert(tc, callback);
        // Caller will use the cid to block on the channel response.
        cid
    }

    pub fn get_send_callback(
        &mut self,
        cid: u64,
    ) -> Option<tokio::sync::oneshot::Receiver<OneShotCallbackMessage>> {
        for (_, callback) in self.send_callbacks.iter_mut() {
            if callback.cid == cid {
                return Some(callback.block_channel.take().unwrap());
            }
        }
        None
    }

    pub fn trigger_send_callback(&mut self, tc: String, response: String) {
        if let Some(callback) = self.send_callbacks.get_mut(&tc) {
            if let Some(channel) = callback.resp_channel.take() {
                let msg = OneShotCallbackMessage {
                    cid: callback.cid,
                    response,
                };
                let result = channel.send(msg);
                if let Err(e) = result {
                    log::error!("Error sending response to callback: {:?}", e);
                } else {
                    log::info!("Triggered callback for tc: {:?}", tc);
                }
            }
        }
    }

    pub fn delete_send_callback(&mut self, _host_sla_idx: ManagedVecU16, tc: String) {
        self.send_callbacks.remove(&tc);
    }
}

impl Default for GlobalsDTPConnsStateClientST {
    fn default() -> Self {
        Self::new()
    }
}
