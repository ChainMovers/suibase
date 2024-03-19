use std::sync::Arc;

use axum::async_trait;

use common::basic_types::GenericChannelMsg;
use dtp_sdk::{Connection, DTP};

use jsonrpsee::core::RpcResult;
use log::info;

use tokio::sync::Mutex;

use crate::admin_controller::{
    AdminControllerMsg, AdminControllerTx, EVENT_NOTIF_CONFIG_FILE_CHANGE,
};
use crate::shared_types::{
    DTPConnStateDataClient, ExtendedWebSocketWorkerIOMsg, Globals, WebSocketWorkerIOMsg,
};
use crate::workers::WebSocketWorker;

use super::RpcInputError;
use super::{DtpApiServer, InfoResponse, PingResponse, RpcSuibaseError};

pub struct DtpApiImpl {
    pub globals: Globals,
    pub admctrl_tx: AdminControllerTx,
}

impl DtpApiImpl {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx) -> Self {
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

        // Convert package id string to an ObjectID.
        if package_id.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("package id not defined".to_string()).into(),
            );
        }
        let package_id = package_id.unwrap();
        info!("Using package_id: {}", package_id);
        // Sanity check the package id.
        if package_id == "0x0000000000000000000000000000000000000000000000000000000000000000" {
            return Err(RpcSuibaseError::InvalidConfig(
                "package id is unexpectedly 0x0".to_string(),
            )
            .into());
        }

        let package_id = dtp_sdk::str_to_object_id(&package_id)
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        if gas_addr.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("gas address not defined".to_string()).into(),
            );
        }
        let gas_addr = dtp_sdk::str_to_sui_address(&gas_addr.unwrap())
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        // Iterate the auths. Create a DTP Client for each, then do the steps to create a Host object (if does not already exists).
        let keystore_path = workdir.path().join("config").join("sui.keystore");

        let mut display_out = String::new();

        for auth in auths {
            let auth_addr = dtp_sdk::str_to_sui_address(&auth)
                .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

            let mut dtp = DTP::new(auth_addr, keystore_path.to_str()).await?;
            dtp.add_rpc_url("http://0.0.0.0:44340").await?;
            dtp.set_package_id(package_id).await;
            dtp.set_gas_address(gas_addr).await;

            // Get localhost for this client, it will be created if does not exists.
            let host = dtp.get_host().await;

            if let Err(e) = host {
                let error_message = format!(
                    "auth addr {} package_id {} inner error [{}]",
                    auth, package_id, e
                );
                return Err(RpcSuibaseError::LocalHostError(error_message).into());
            }

            let host = host.unwrap();

            // Send a message to WebSocketWorker to monitor this Host for events (if not already done).
            {
                // Get the target channel if it exists.
                let channel = {
                    let channels_guard = self.globals.get_channels(workdir_idx).read().await;
                    let channels = &*channels_guard;
                    channels.to_websocket_worker_io.clone()
                };
                if let Some(channel) = channel {
                    let mut msg = GenericChannelMsg::new();
                    msg.event_id = common::basic_types::EVENT_EXEC;
                    msg.command = Some("localhost_update".to_string());
                    let ext_msg = ExtendedWebSocketWorkerIOMsg {
                        generic: msg,
                        localhost: Some(host.clone()),
                        ..Default::default()
                    };
                    let ws_msg = WebSocketWorkerIOMsg::Extended(ext_msg);
                    let _ = channel.send(ws_msg).await;
                }
            }

            // Display the alias and the host address.
            display_out.push_str(&format!(
                "Auth address: {} Host Object ID: {}\n",
                auth,
                host.object_id()
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
        let package_id = package_id.unwrap();
        // Sanity check the package id.
        if package_id == "0x0000000000000000000000000000000000000000000000000000000000000000" {
            return Err(RpcSuibaseError::InvalidConfig(
                "package id is unexpectedly 0x0".to_string(),
            )
            .into());
        }

        let package_id = dtp_sdk::str_to_object_id(&package_id)
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        // Convert gas_addr string to a SuiAddress.
        if gas_addr.is_none() {
            return Err(
                RpcSuibaseError::InvalidConfig("gas address not defined".to_string()).into(),
            );
        }
        let gas_addr = dtp_sdk::str_to_sui_address(&gas_addr.unwrap())
            .map_err(|e| RpcSuibaseError::InvalidConfig(e.to_string()))?;

        // Convert host_addr string to an ObjectID.
        let host_id = dtp_sdk::str_to_object_id(&host_addr)
            .map_err(|e| RpcInputError::InvalidParams("host_addr".to_string(), e.to_string()))?;

        // Variables initialized while holding the GlobalsDTPConnsState mutex.
        let mut need_to_get_localhost = false;
        let mut dtp_access: Option<Arc<Mutex<DTP>>> = None;
        let mut host_sla_idx: Option<u16> = None;
        let mut conn: Option<Connection> = None;

        {
            let mut conns_state_guard = self.globals.dtp_conns_state(workdir_idx).write().await;
            let conns_state = &mut *conns_state_guard;

            let mut conn_data: Option<&DTPConnStateDataClient> = None;
            host_sla_idx = conns_state.conns.get_if_some(7, &host_addr, 0);

            if let Some(host_sla_idx) = host_sla_idx {
                // Get the existing DtpConnStateData.
                conn_data = conns_state.conns.get(host_sla_idx);
                if conn_data.is_none() {
                    return Err(RpcSuibaseError::InternalError(
                        "Connection data unexpectedly missing".to_string(),
                    )
                    .into());
                } else {
                    let existing_conn_data = conn_data.unwrap();
                    //TODO if existing_conn_data.is_open { initialize conn }
                    if existing_conn_data.dtp.is_some() {
                        // Get the existing DTP.
                        dtp_access = Some(Arc::clone(existing_conn_data.dtp.as_ref().unwrap()));
                    } else {
                        return Err(RpcSuibaseError::InternalError(
                            "DTP client unexpectedly missing".to_string(),
                        )
                        .into());
                    }
                }
            } else {
                // Need to create the DTP and DtpConnStateData.
                let keystore_path = workdir.path().join("config").join("sui.keystore");
                let mut new_dtp = DTP::new(gas_addr, keystore_path.to_str()).await?;

                // TODO Remove hard coding
                new_dtp.add_rpc_url("http://0.0.0.0:44340").await?;
                new_dtp.set_gas_address(gas_addr).await;
                new_dtp.set_package_id(package_id).await;
                dtp_access = Some(Arc::new(Mutex::new(new_dtp)));

                let mut new_conn_state = DTPConnStateDataClient::new();
                new_conn_state.set_dtp(dtp_access.as_ref().unwrap());
                host_sla_idx = conns_state
                    .conns
                    .push(new_conn_state, 7, host_addr.clone(), 0);
                if host_sla_idx.is_none() {
                    return Err(RpcSuibaseError::InternalError(
                        "Max number of connections reached".to_string(),
                    )
                    .into());
                }
                // Further network action performed outside the Mutex.
                need_to_get_localhost = true;
            }
        };

        if host_sla_idx.is_none() {
            return Err(RpcSuibaseError::InternalError(
                "Unexpected host_sla_idx not initialized".to_string(),
            )
            .into());
        }
        let host_sla_idx = host_sla_idx.unwrap();

        if dtp_access.is_none() {
            return Err(RpcSuibaseError::InternalError(
                "Unexpected DTP client not initialized".to_string(),
            )
            .into());
        }
        let dtp_access = dtp_access.unwrap();

        // Make sure the localhost exists (created as needed).
        // Note: We don't need the API 'Host' handle on it.
        if need_to_get_localhost {
            let mut dtp = dtp_access.lock().await;
            let host = dtp.get_host().await;

            if let Err(e) = host {
                let error_message = format!("package_id {} inner error {}", package_id, e);
                return Err(RpcSuibaseError::LocalHostError(error_message).into());
            }
        };

        // Get the API handle on the remote host.
        let target_host = {
            let dtp = dtp_access.lock().await;
            info!("In API doing get_host_by_id for host_id: {:?}", host_id);
            dtp.get_host_by_id(host_id).await?
        };

        // The remote host must exist!
        if target_host.is_none() {
            return Err(RpcSuibaseError::RemoteHostDoesNotExists(host_addr).into());
        }
        let target_host = target_host.unwrap();

        // If connection not open, try to recover/create one.
        if conn.is_none() {
            let mut dtp = dtp_access.lock().await;
            let open_conn = dtp.create_connection(&target_host, 7).await;
            if let Err(e) = open_conn {
                let error_message = format!("package_id {} inner error {}", package_id, e);
                return Err(RpcSuibaseError::ConnectionCreationFailed(error_message).into());
            }
            conn = Some(open_conn.unwrap());
        }

        // Tell the WebSocketWorker to monitor this connection.
        // Verify with globals if WebSocketWorker is monitoring the connection.
        // If not, spin re-checking every ~50ms for up to 1 second.

        // Call into DTP to send data (will return a request handle).

        // Wait block for a response using the request handle.
        // (the handle is just a one-shot channel with timeout).

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
