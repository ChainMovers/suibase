use axum::async_trait;

use jsonrpsee::core::RpcResult;

use crate::admin_controller::{
    AdminControllerMsg, AdminControllerTx, EVENT_NOTIF_CONFIG_FILE_CHANGE,
};
use crate::shared_types::{DTPConnStateData, Globals};

use super::{DtpApiServer, InfoResponse, PingResponse, RpcSuibaseError};
use super::{RpcInputError};

pub struct DtpApiImpl {
    pub globals: Globals,
    /*pub globals_conns_state: GlobalsDTPConnsStateMT,
    pub globals_conns_state_tx: GlobalsDTPConnsStateTxMT,
    pub globals_conns_state_rx: GlobalsDTPConnsStateRxMT,*/
    pub admctrl_tx: AdminControllerTx,
}

impl DtpApiImpl {
    pub fn new(
        globals: Globals,
        /*globals_conns_state: GlobalsDTPConnsStateMT,
        globals_conns_state_tx: GlobalsDTPConnsStateTxMT,
        globals_conns_state_rx: GlobalsDTPConnsStateRxMT,*/
        admctrl_tx: AdminControllerTx,
    ) -> Self {
        Self {
            globals,
            admctrl_tx,
        }
    }
}

#[async_trait]
impl DtpApiServer for DtpApiImpl {
    async fn fs_change(&self, path: String) -> RpcResult<InfoResponse> {
        let mut resp = InfoResponse::new();

        // Initialize some of the header fields.
        resp.header.method = "fsChange".to_string();

        // Inform the AdminController that something changed...
        let mut msg = AdminControllerMsg::new();
        msg.event_id = EVENT_NOTIF_CONFIG_FILE_CHANGE;
        msg.data_string = Some(path);

        // TODO: Implement response to handle errors... but is it really needed here?
        let _ = self.admctrl_tx.send(msg).await;

        resp.info = "Success".to_string();
        Ok(resp)
    }

    async fn ping(
        &self,
        workdir: String,
        host_addr: String,
        bytes: Option<String>,
    ) -> RpcResult<PingResponse> {
        let mut resp = PingResponse::new();

        // Initialize some of the header fields.
        resp.header.method = "ping".to_string();

        let workdir_idx = match self.globals.get_workdir_idx_by_name(&workdir)
            .await
        {
            Some(workdir_idx) => workdir_idx,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Get the default DTP client address from the suibase.yaml or active address (for this workdir).

        let (host_sla_idx, is_open) = {
            // Get the HostSlaIdx (will be created if does not exists).
            let mut conns_state_guard = self.globals.dtp_conns_state(workdir_idx).write().await;
            let conns_state = &mut *conns_state_guard;
            let mut host_sla_idx = conns_state.conns.get_if_some(7, &host_addr, 0);
            if host_sla_idx.is_none() {
                // Create it.
                let new_conn_state = DTPConnStateData::new();
                host_sla_idx = conns_state.conns.push(new_conn_state, 7, host_addr, 0);
                if host_sla_idx.is_none() {
                    return Err(RpcSuibaseError::InternalError(
                        "Max number of connections reached".to_string(),
                    )
                    .into());
                }
            }
            let host_sla_idx = host_sla_idx.unwrap();
            let conn_data = conns_state.conns.get(host_sla_idx);
            if conn_data.is_none() {
                return Err(RpcSuibaseError::InternalError(
                    "Connection data unexpectedly missing".to_string(),
                )
                .into());
            }
            let conn_data = conn_data.unwrap();
            (host_sla_idx, conn_data.is_open)
        };

        if !is_open {
            // Send request to WebSocketTXWorker to open the connection.
            // TODO TODO TODO
        }

        // Connection not open, try to open it.

        // TODO: Is the server healthy?

        // A mix of RWLock protected ManagedVec and message passing is used for multi-threading
        //
        // Data Sharing (all ManagedVec):
        //    ConnsState[HostSlaIdx]: Shared between API and WebSocketTxWorker for open/close state.
        //    ConnsStateTX[HostSlaIdx]: Shared between API threads only when preparing to send data.
        //    ConnsStateRX[HostSlaIdx]: Not shared for now. Used by WebSocketRxWorker only.
        //    PendingRequest[HostSlaIdx]: Shared between API and WebSocketRxWorker.
        //
        // Short-term strategy for coarse locking of ManagedVec as a whole:
        //   Write Lock on ManagedVec<data>
        //      Perform write operations on data.
        //          And/Or
        //       Modify array itself
        //   Write Unlock
        //
        // Long-term strategy for finer locking of ManagedVec elements:
        //   Write Lock on ManagedVec<data>:
        //     Arc increment data.mutex (useable outside this critical section).
        //          And/Or
        //     Modify array itself
        //   Write Unlock
        //
        //   Write Lock the Arc<Mutex<data>>:
        //      Perform write operations on data
        //   Write Unlock
        //
        //   Let Arc<Mutex<data>> decrement when exiting scope.
        //
        //
        // Data Flow Request Send:
        //   API --(MsgQ)--> WebSocketWorker --(RPC)--> Sui Network
        //    ^                     |
        //    |                     |
        //    +--(OneShot Result)---+
        //
        // Data Flow Response Receive:
        //   Sui Network --(RPC)--> WebSocketWorker --(MsgQ)--> WebSocketRxWorker --(OneShot Data)--> API
        //
        //
        // **** API Thread - Request Sequence ****
        //
        //   (1) Find the HostSlaIdx.
        //
        //       HostSlaIdx is a U16 index unique for each (service_idx,host_addr,sla_idx) key tuple. See managed_vec_map_vec.rs.
        //
        //   (2) As needed, the tx thread has to open the connection:
        //     Write lock on ConnsState[HostSlaIdx]
        //       Find if open needed.
        //     Write unlock
        //     if open needed, send request to WebSocketWorker.
        //     OneShot block until confirmed open or timeout.
        //
        //   (3) Prepare for TX:
        //       Write lock the TXController[HostSlaIdx]
        //         Find the proper IPipe and sequence number.
        //         Run the local TX state machine
        //       Write unlock
        //       if sync protocol needed, send trigger to WebSocketWorker (do not wait for completion).
        //
        //   (4) Prepare RX side to expect a response:
        //       Write lock the RXController[HostSlaIdx]
        //         Add PendingRequest to it.
        //       Write unlock
        //
        //   (5) Send data to WebSocketWorker (with IPipe, SeqNumber info).
        //       OneShot block for the response, failure or timeout
        //
        //
        // **** WebSocketWorker ****
        //   - On open connection request, do
        //       On open needed:
        //         Call into Sui network to open the connection (until success or timeout).
        //         Do RPC subscriptions (until success or timeout).
        //         Create TXController[HostSlaIdx] and RXController[HostSlaIdx].
        //         Write lock ConnsController[HostSlaIdx]
        //           Mark the connection as open.
        //         Write Unlock
        //     Send success/failure to caller with oneshot channel. Success if already open.
        //
        //   - On Exec Move Call (a oneshot chanel is provided)
        //       Send the requested operation on the websocket.
        //       On success:
        //         Write Lock RXControllerMoveCalls[HostSlaIdx]
        //           Add PendingRequest and move ownership of oneshot channel.
        //         Write Unlock

        //       On failure:
        //         Send failure to caller with oneshot channel.
        //
        // **** WebSocketRxWorker ****
        //  On peer data receive:
        //    Use the subscription ID to find HostSlaIdx (slow lookup).
        //    (Optimization: Response could include the HostSlaIdx for quick tentative lookup)
        //
        //    Write lock RXController[HostSlaIdx]
        //      Run RX State machine.
        //      Drop if invalid or no pending request.
        //      If valid, take ownership of the oneshot channel and delete PendingRequest.
        //    Write unlock.
        //    If own the oneshot channel, then send the response with the data.
        //
        //  On subscription-level event:
        //    Use the subscription ID to find HostSlaIdx (slow lookup).
        //    Run subscription state machine (does its own TX as needed)
        //
        //  On Move Call response:
        //    Use the returned HostSlaIdx for fast lookup.
        //    Write lock RXControllerMoveCalls[HostSlaIdx]
        //      Drop if invalid or no pending request.
        //      If valid, take ownership of the oneshot channel and delete PendingRequest.
        //    Write unlock.
        //    If own the oneshot channel, then send the response with the data.

        Ok(resp)
    }
}
