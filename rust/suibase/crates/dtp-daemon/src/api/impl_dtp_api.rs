use std::f64::consts::E;
use std::str::FromStr;

use axum::async_trait;

use axum::extract::Host;
use dtp_sdk::DTP;
use hyper::client;
use jsonrpsee::core::RpcResult;
use log::info;
use serde_with::hex;
use sui_types::base_types::{ObjectID, SuiAddress};

use crate::admin_controller::{
    AdminControllerMsg, AdminControllerTx, EVENT_NOTIF_CONFIG_FILE_CHANGE,
};
use crate::shared_types::{DTPConnStateData, Globals};

use super::RpcInputError;
use super::{DtpApiServer, InfoResponse, PingResponse, RpcSuibaseError};

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

    async fn publish(
        &self,
        workdir: String,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
    ) -> RpcResult<InfoResponse> {
        // Common pattern used for controlling the output.
        let debug = debug.unwrap_or(false);
        let display = display.unwrap_or(debug);
        let data = data.unwrap_or(!(debug || display));

        let mut debug_out = String::new();
        let mut display_out = String::new();
        let mut data_out = String::new();

        // Apply the suibase.yaml configuration.
        //
        // Make sure all Hosts under local authority exists on the network.
        //
        // If they exists, update them as needed.
        //
        // Response includes the address of all owned Host object.
        let mut resp = InfoResponse::new();

        // Initialize some of the header fields.
        resp.header.method = "publish".to_string();

        // TODO This need to be optimized (probably merge into GlobalsWorkdirConfigST)
        let (workdir_idx, workdir) = match self.globals.get_workdir_by_name(&workdir).await {
            Some(x) => x,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Iterate the WorkdirConfig DTP services. Identify every unique client_auth and server_auth.
        let mut auths = Vec::<String>::new();

        let (gas_addr, package_id) = {
            let globals_guard = self.globals.get_config(workdir_idx).read().await;
            let config = &*globals_guard;
            let mut gas_addr = config.user_config.dtp_default_gas_address();
            let dtp_services = config.user_config.dtp_services();
            for dtp_service in dtp_services {
                let client_auth = dtp_service.client_auth();
                let server_auth = dtp_service.server_auth();
                // Put the auth strings in a vector<String>, where the string is the client_auth.to_string or
                // server_auth.to_string if not already in the vector.
                if let Some(client_auth) = client_auth {
                    if !auths.contains(client_auth) {
                        auths.push(client_auth.clone());
                    }
                    if gas_addr.is_none() && dtp_service.gas_address().is_some() {
                        gas_addr = dtp_service.gas_address().cloned();
                    }
                }
                if let Some(server_auth) = server_auth {
                    if !auths.contains(server_auth) {
                        auths.push(server_auth.clone());
                    }
                }
            }
            (gas_addr, config.user_config.dtp_package_id())
        };

        if package_id.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("package id not defined".to_string()).into(),
            );
        }
        let package_id = dtp_sdk::str_to_object_id(&package_id.unwrap())
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        if gas_addr.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("gas address not defined".to_string()).into(),
            );
        }
        let gas_addr = dtp_sdk::str_to_sui_address(&gas_addr.unwrap())
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        // Iterate the auths. Create a DTP Client for each, then do the steps to create a Host object (if does not already exists).
        let keystore_path = workdir
            .path()
            .join("config".to_string())
            .join("sui.keystore");

        let mut display_out = String::new();

        for auth in auths {
            let auth_addr = dtp_sdk::str_to_sui_address(&auth)
                .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

            let mut dtp = DTP::new(auth_addr, keystore_path.to_str()).await?;
            dtp.add_rpc_url("http://0.0.0.0:44340").await?;
            dtp.set_package_id(package_id);
            dtp.set_gas_address(gas_addr);

            // Get localhost for this client, it will be created if does not exists.
            let host = dtp.get_host().await;

            if let Err(_) = host {
                if let Err(e) = host {
                    let error_message = format!(
                        "auth addr {} package_id {} inner error [{}]",
                        auth,
                        package_id.to_string(),
                        e.to_string()
                    );
                    return Err(RpcSuibaseError::LocalHostError(error_message).into());
                }
            }
            let host = host.unwrap();

            // Display the alias and the host address.
            display_out.push_str(&format!(
                "Auth address: {} Host Object ID: {}\n",
                auth,
                host.id()
            ));
            if debug {
                debug_out.push_str(&format!("Host={:?}\n", host));
            }
        }
        if data && !data_out.is_empty() {
            resp.data = Some(data_out);
        }
        if display && !display_out.is_empty() {
            resp.display = Some(display_out);
        }
        if debug && !debug_out.is_empty() {
            resp.debug = Some(debug_out);
        }
        resp.info = "Success".to_string();
        Ok(resp)
    }

    async fn ping(
        &self,
        workdir: String,
        host_addr: String,
        _bytes: Option<String>,
        data: Option<bool>,
        display: Option<bool>,
        debug: Option<bool>,
    ) -> RpcResult<PingResponse> {
        // Common pattern used for controlling the output.
        let debug = debug.unwrap_or(false);
        let display = display.unwrap_or(debug);
        let data = data.unwrap_or(!(debug || display));

        let mut debug_out = String::new();
        let mut display_out = String::new();
        let mut data_out = String::new();

        let mut resp = PingResponse::new();

        // Initialize some of the header fields.
        resp.header.method = "ping".to_string();

        // TODO This need to be optimized (probably merge into GlobalsWorkdirConfigST)
        let (workdir_idx, workdir) = match self.globals.get_workdir_by_name(&workdir).await {
            Some(x) => x,
            None => return Err(RpcInputError::InvalidParams("workdir".to_string(), workdir).into()),
        };

        // Get the default and ping specific client address from the suibase.yaml.
        let (gas_addr, service_config, package_id) = {
            let globals_guard = self.globals.get_config(workdir_idx).read().await;
            let config = &*globals_guard;
            let default_gas_addr = config.user_config.dtp_default_gas_address();
            let service_config = config.user_config.dtp_service_config(7, None);
            let package_id = config.user_config.dtp_package_id();
            (default_gas_addr, service_config, package_id)
        };

        info!(
            "ping: gas_addr: {:?} for workdir_idx: {:?} and workdir: {:?}",
            gas_addr, workdir_idx, workdir
        );

        // If service_config is None return an error.
        if service_config.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("ping service not defined".to_string()).into(),
            );
        }
        let service_config = service_config.unwrap();

        // If service_config is not enabled, return an error.
        if !service_config.is_enabled() {
            return Err(
                RpcSuibaseError::InvalidConfig("ping service is disabled".to_string()).into(),
            );
        }

        // Convert package id string to an ObjectID.
        if package_id.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("package id not defined".to_string()).into(),
            );
        }
        let package_id = dtp_sdk::str_to_object_id(&package_id.unwrap())
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        // Convert gas_addr string to a SuiAddress.
        if gas_addr.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("gas address not defined".to_string()).into(),
            );
        }
        let gas_addr = dtp_sdk::str_to_sui_address(&gas_addr.unwrap())
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        let (host_sla_idx, is_open) = {
            // Get the HostSlaIdx (will be created if does not exists).
            let mut conns_state_guard = self.globals.dtp_conns_state(workdir_idx).write().await;
            let conns_state = &mut *conns_state_guard;
            // TODO: Validate host_addr before insertion.
            let mut host_sla_idx = conns_state.conns.get_if_some(7, &host_addr, 0);
            if host_sla_idx.is_none() {
                // Create a DTP client for it.
                let keystore_path = workdir
                    .path()
                    .join("config".to_string())
                    .join("sui.keystore");
                let mut dtp = DTP::new(gas_addr, keystore_path.to_str()).await?;

                // TODO Remove hard coding
                dtp.add_rpc_url("http://0.0.0.0:44340").await?;
                dtp.set_package_id(package_id);

                // Make sure localhost exists for this client.
                let host = dtp.get_host().await;

                if let Err(e) = host {
                    let error_message = format!(
                        "package_id {} inner error {}",
                        package_id.to_string(),
                        e.to_string()
                    );
                    return Err(RpcSuibaseError::LocalHostError(error_message).into());
                }
                let host = host.unwrap();

                let mut new_conn_state = DTPConnStateData::new();
                new_conn_state.set_dtp(dtp);
                new_conn_state.set_localhost(host);
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
        if data && !data_out.is_empty() {
            resp.data = Some(data_out);
        }
        if display && !display_out.is_empty() {
            resp.display = Some(display_out);
        }
        if debug && !debug_out.is_empty() {
            resp.debug = Some(debug_out);
        }
        Ok(resp)
    }
}
