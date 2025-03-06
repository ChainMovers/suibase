use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use common::{basic_types::*, log_safe};
use fastcrypto::{
    ed25519::Ed25519KeyPair,
    encoding::{Base58, Encoding},
    traits::{KeyPair, ToFromBytes},
};

use crate::shared_types::{GlobalsWorkdirConfigMT, GlobalsWorkdirStatusMT};

use anyhow::{anyhow, Context, Result};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

// Design
//
// Up to 25x20MB files are stored in ~/suibase/workdirs/common/autocoins/storage
// Each file is downloaded from the POI server.
//
// Periodically (once a day) the ACoinsMon will run the proof-of-installation(POI) protocol
// with the POI server.
//
// The protocol is over JSON-RPC.
//
// POI Protocol
// ============
// Open a TLS connection with the POI server at url https://poi.suibase.io
//
// JSON-RPC Method: ChallengeRequest
// data:
//   {
//     "version": 1  // Protocol version
//     "pk": "ajdh26yash"  // user.keypair public key in base58
//   }
//
// The POI server will then respond with a signature challenge in JSON:
//   {
//     "version": 1,    // Protocol version
//     "file_id": 1,    // File to read from (the id is used in the filename).
//     "offset": 192928, // Byte offset to read within the file. From 0 to 20MB - 256 bytes
//     "length": 43,    // Number of bytes to read in the file. From 32 to 256 bytes
//   }
//
// The ACoinsMonitor will then handle build and send the challenge response with another method:
// JSON-RPC Method: ChallengeResponse
// data:
//   {
//     "version": 1,      // Protocol version.
//     "signature": "asdfasdf" // Base58 encoded response to the challenge.
//                             // 0 has the special meaning that the file is not present or corrupted.
//   }
//
// signature is calculated as follow:
//    - Open for read the file ~/suibase/workdirs/common/autocoins/storage/{user.keypair public key in base58}/{file_nb}.data
//    - Read the bytes at offset and length in a buffer and calculate its signature with user.keypair public key.
//
//    Signature are done with the ed25519 algorithm from the fastcrypto library.
//
// The POI server will then respond with a JSON-RPC message:
//   {
//     "version": 1,    // Protocol version
//     "result": "ok"   // or "error"
//     "file_id": 1,    // File to download (this is the id used in the filename). 0 means no file to download.
//     "download_id": "" // Ephemeral Base58 url used to download the file. Present only if a file should be downloaded.
//   }
//
// The ACoinsMonitor will then immediatly initiate the download of the file using the download_id with:
//    https://poi.suibase.io/download/{download_id}
//
// The download_id a URL encode the following:
//   - UTC expiration time (usually 30 secs after the download_id was created).
//   - user.keypair public key
//   - file_id to generate
//   - signed by the POI server for tampering detection
//
// If failure, then will have to wait 24 hours before trying again with a new challenge.

pub struct ACoinsMonMsg {
    // Internal messaging. Sent for every user request/response.
    // Purposely pack this in a few bytes for performance reason.
    event_id: ACoinsMonEvents,
}

impl ACoinsMonMsg {
    pub fn new() -> Self {
        Self { event_id: 0 }
    }
}

// Events ID.
// See GenericChannelID for guidelines to set these values.
pub type ACoinsMonEvents = u8;

impl std::fmt::Debug for ACoinsMonMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ACoinsMonMsg {{ event_id: {} }}", self.event_id)
    }
}

pub type ACoinsMonTx = tokio::sync::mpsc::Sender<ACoinsMonMsg>;
pub type ACoinsMonRx = tokio::sync::mpsc::Receiver<ACoinsMonMsg>;

pub struct ACoinsMonitor {
    globals_devnet_config: GlobalsWorkdirConfigMT,
    globals_testnet_config: GlobalsWorkdirConfigMT,
    globals_devnet_status: GlobalsWorkdirStatusMT,
    globals_testnet_status: GlobalsWorkdirStatusMT,
    acoinsmon_rx: ACoinsMonRx,
    init_time: SystemTime,
    last_audit_time: SystemTime,
}

struct ACoinsMonAuditVar {
    // Variables that exists only for the time that the ACoinsMonitor is
    // running an audit.
    user_keypair: Ed25519KeyPair,
    user_pk_bytes: Vec<u8>,
    user_pk_base58: String,
    last_verification_success_time: SystemTime,
    last_verification_fail_time: SystemTime,
}

// This is how the ProxyHandler communicate with the NetworkMonitor.
impl ACoinsMonitor {
    pub fn new(
        globals_devnet_config: GlobalsWorkdirConfigMT,
        globals_testnet_config: GlobalsWorkdirConfigMT,
        globals_devnet_status: GlobalsWorkdirStatusMT,
        globals_testnet_status: GlobalsWorkdirStatusMT,
        acoinsmon_rx: ACoinsMonRx,
    ) -> Self {
        Self {
            globals_devnet_config,
            globals_testnet_config,
            globals_devnet_status,
            globals_testnet_status,
            acoinsmon_rx,
            init_time: SystemTime::now(),
            last_audit_time: SystemTime::UNIX_EPOCH,
        }
    }

    pub async fn send_event_audit(tx_channel: &ACoinsMonTx) -> Result<()> {
        let mut msg = ACoinsMonMsg::new();
        msg.event_id = EVENT_AUDIT;
        tx_channel.send(msg).await.map_err(|e| {
            log::debug!("failed {}", e);
            anyhow!("failed {}", e)
        })
    }

    // Function that delete all files in a directory, except for the ones starting with the specified prefixes.

    async fn delete_old_files_except_prefixes(path: &Path, prefixes: Vec<&str>) -> Result<()> {
        let mut entries = tokio::fs::read_dir(path)
            .await
            .map_err(|e| anyhow!("failed to read directory {}: {}", path.display(), e))?;

        while let Some(entry) = entries.next_entry().await? {
            let file_path = entry.path();
            if file_path.is_file() {
                let file_name = match file_path.file_name().and_then(|name| name.to_str()) {
                    Some(name) => name,
                    None => continue,
                };

                let should_delete = !prefixes.iter().any(|prefix| file_name.starts_with(prefix));
                if should_delete {
                    tokio::fs::remove_file(&file_path).await.map_err(|e| {
                        anyhow!("failed to delete file {}: {}", file_path.display(), e)
                    })?;
                }
            }
        }
        Ok(())
    }

    async fn read_timestamp_from_file(path: &Path) -> Result<SystemTime> {
        let timestamp_str = tokio::fs::read_to_string(path).await.context(format!(
            "failed to read timestamp from file {}",
            path.display()
        ))?;
        let timestamp = timestamp_str
            .parse::<u64>()
            .context("failed to parse timestamp from string")?;
        Ok(UNIX_EPOCH + Duration::from_secs(timestamp))
    }

    async fn write_timestamp_to_file(path: &Path) -> Result<()> {
        let now = SystemTime::now();
        let duration_since_epoch = now
            .duration_since(UNIX_EPOCH)
            .context("Time went backwards")?
            .as_secs();
        let timestamp_str = duration_since_epoch.to_string();
        tokio::fs::write(path, timestamp_str).await.context(format!(
            "failed to write timestamp to file {}",
            path.display()
        ))
    }

    async fn audit_init(&mut self) -> Result<ACoinsMonAuditVar> {
        // Each installation have a user.keypair file created in ~/suibase/workdirs/common/autocoins
        //
        // It is an ed25519 keypair created with fastcrypto from Mysten Labs and used later to
        // sign/authenticate when connecting to the POI server.
        //
        // If the user change or delete that keypair, then the storage becomes useless (will need to
        // recover/redownload from the POI server).
        //
        let path = common::shared_types::get_workdir_common_path().join("autocoins");
        let user_keypair_file = path.join("user.keypair");
        let mut user_keypair: Option<Ed25519KeyPair> = None;

        // Atempt, up to 3 times, to get a user.keypair file verified and loaded.
        let attempts = 0;
        let mut verified_invalid = false;
        while attempts < 3 && user_keypair.is_none() {
            // Delete a user.keypair file it it was verified invalid (in a previous iteration).
            if verified_invalid {
                if user_keypair_file.exists() {
                    if let Err(error) = tokio::fs::remove_file(&user_keypair_file).await {
                        let err_msg = format!(
                            "failed to delete file {}: {}",
                            user_keypair_file.display(),
                            error
                        );
                        log_safe!(err_msg);
                        continue;
                    }
                }
                verified_invalid = false;
            }

            if !user_keypair_file.exists() {
                // Create a new user.keypair file.
                if let Err(error) = tokio::fs::create_dir_all(&path).await {
                    let err_msg =
                        format!("failed to create directory {}: {}", path.display(), error);
                    log_safe!(err_msg);
                    continue;
                }

                if !user_keypair_file.exists() {
                    let keypair = Ed25519KeyPair::generate(&mut rand::thread_rng());
                    let keypair_bytes = keypair.as_bytes();
                    let keypair_base58 = Base58::encode(keypair_bytes);
                    if let Err(error) = tokio::fs::write(&user_keypair_file, &keypair_base58).await
                    {
                        let err_msg = format!(
                            "failed to write keypair to file {}: {}",
                            user_keypair_file.display(),
                            error
                        );
                        log_safe!(err_msg);
                        continue;
                    }
                }
            }

            // Read and validate the user.keypair file
            let user_keypair_base58 = match tokio::fs::read_to_string(&user_keypair_file).await {
                Ok(user_keypair_base58) => user_keypair_base58,
                Err(error) => {
                    let err_msg = format!(
                        "failed to read keypair from file {}: {}",
                        user_keypair_file.display(),
                        error
                    );
                    log_safe!(err_msg);
                    verified_invalid = true;
                    continue;
                }
            };

            let decode_results = Base58::decode(&user_keypair_base58);
            if let Err(error) = decode_results {
                let err_msg = format!(
                    "failed to decode keypair (1) from file {}: {}",
                    user_keypair_file.display(),
                    error
                );
                log_safe!(err_msg);
                verified_invalid = true;
                continue;
            }

            if let Ok(user_keypair_bytes) = &decode_results {
                user_keypair = match Ed25519KeyPair::from_bytes(user_keypair_bytes) {
                    Ok(user_keypair) => Some(user_keypair),
                    Err(error) => {
                        let err_msg = format!(
                            "failed to decode keypair (2) from file {}: {}",
                            user_keypair_file.display(),
                            error
                        );
                        log_safe!(err_msg);
                        verified_invalid = true;
                        continue;
                    }
                };
            }
        }

        if user_keypair.is_none() {
            let err_msg = "failed to load user keypair";
            log_safe!(err_msg);
            return Err(anyhow!(err_msg));
        }

        let user_keypair = user_keypair.unwrap();
        let user_pk_bytes = user_keypair.public().as_bytes();
        let user_pk_base58 = Base58::encode(user_pk_bytes);
        let user_pk_bytes = user_pk_bytes.to_vec();

        // Create data subdirectory and clean-it up base on what is stored in user.keypair.
        let path_data = path.join("data");
        if !path_data.exists() {
            // Create path if does not exists.
            if let Err(error) = tokio::fs::create_dir_all(&path_data).await {
                let err_msg = format!(
                    "failed to create directory {}: {}",
                    path_data.display(),
                    error
                );
                log_safe!(err_msg);
                return Err(anyhow!(err_msg));
            }
        } else {
            // Delete all files in the data directory that are not prefixed by the user keypair public key.
            let prefixes: Vec<&str> = vec![&user_pk_base58];
            if let Err(error) = Self::delete_old_files_except_prefixes(&path_data, prefixes).await {
                let err_msg = format!(
                    "failed to delete old files in directory {}: {}",
                    path_data.display(),
                    error
                );
                log_safe!(err_msg);
                return Err(anyhow!(err_msg));
            }
        }

        // Read EpochTimestamps from files (when existing).
        let path_last_verification_success_time = path.join("last_verification_success_time");
        let last_verification_success_time =
            match Self::read_timestamp_from_file(&path_last_verification_success_time).await {
                Ok(last_verification_success_time) => last_verification_success_time,
                Err(error) => {
                    let err_msg = format!(
                        "failed to read last_verification_success_time from file {}: {}",
                        path_last_verification_success_time.display(),
                        error
                    );
                    log_safe!(err_msg);
                    SystemTime::UNIX_EPOCH
                }
            };

        let path_last_verification_fail_time = path.join("last_verification_fail_time");
        let last_verification_fail_time =
            match Self::read_timestamp_from_file(&path_last_verification_fail_time).await {
                Ok(last_verification_fail_time) => last_verification_fail_time,
                Err(error) => {
                    let err_msg = format!(
                        "failed to read last_verification_fail_time from file {}: {}",
                        path_last_verification_fail_time.display(),
                        error
                    );
                    log_safe!(err_msg);
                    SystemTime::UNIX_EPOCH
                }
            };

        Ok(ACoinsMonAuditVar {
            user_keypair,
            user_pk_bytes,
            user_pk_base58,
            last_verification_success_time,
            last_verification_fail_time,
        })
    }

    async fn process_msg(&mut self, msg: ACoinsMonMsg) {
        let mut audit_var: Option<ACoinsMonAuditVar> = None;

        {
            //let globals_read_guard = self.globals_testnet_status.read().await;
            //let globals = &*globals_read_guard;
            match msg.event_id {
                EVENT_AUDIT => {
                    // Use last_audit_time to make sure never audit more than once per hour.
                    // TODO Switch this to 3600 for 1 hour. For now, just 1 second while developing.
                    let now = SystemTime::now();
                    if now.duration_since(self.last_audit_time).unwrap() < Duration::from_secs(1) {
                        return;
                    }
                    self.last_audit_time = now;
                    if let Ok(vars) = self.audit_init().await {
                        audit_var = Some(vars);
                    }
                }
                _ => {
                    log::debug!("process_msg unexpected event id {}", msg.event_id);
                    return;
                }
            }
        }

        if audit_var.is_none() {
            return;
        }
        let audit_var = audit_var.unwrap();

        // Skip further operation if verfification:
        //   - was already done successfully in last 23 hours.
        //   - was already done unsuccessfully in last 12 hours.
        let now = SystemTime::now();
        if now
            .duration_since(audit_var.last_verification_success_time)
            .unwrap()
            < Duration::from_secs(1)
        // TODO Switch this to 24 * 60 * 60
        {
            return;
        }
        if now
            .duration_since(audit_var.last_verification_fail_time)
            .unwrap()
            < Duration::from_secs(1)
        // TODO Switch this to 12 * 60 * 60
        {
            return;
        }
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            let cur_msg = self.acoinsmon_rx.recv().await;
            if cur_msg.is_none() || subsys.is_shutdown_requested() {
                // Channel closed or shutdown requested.
                return;
            }
            self.process_msg(cur_msg.unwrap()).await;
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

        // The loop to handle all incoming messages.
        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

/*
    async fn process_mut_globals(&mut self, msg: NetmonMsg) -> Option<NetmonMsg> {
        // Process messages that requires WRITE access to the globals.
        //
        // All the NetmonMsg are process single threaded (by the netmon thread).
        //
        if !msg.flags.intersects(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX) {
            // Do not consume the message.
            return Some(msg);
        }

        {
            let mut globals_write_guard = self.globals.write().await;
            let globals = &mut *globals_write_guard;
            let input_ports = &mut globals.input_ports;

            let mut cur_msg = msg;
            loop {
                match cur_msg.event_id {
                    EVENT_REPORT_TGT_REQ_RESP_OK => {
                        // Update the stats. Consume the message.
                        if cur_msg
                            .flags
                            .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                        {
                            // This is for the "controlled" latency test.
                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                target_server
                                    .stats
                                    .handle_latency_report(cur_msg.timestamp, cur_msg.para32[1]);

                                // Always update the selection_vectors on a good latency_report. This is
                                // the periodic "audit" opportunity to refresh things up.
                                Self::update_selection_vectors(input_ports, &cur_msg);
                            }
                        } else {
                            // This is for the user traffic.
                            if let Some(stats) = crate::NetworkMonitor::get_mut_all_servers_stats(
                                input_ports,
                                &cur_msg,
                            ) {
                                stats.handle_resp_ok(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                );
                            }

                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                target_server.stats.handle_resp_ok(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                );
                            }
                        }
                    }
                    EVENT_REPORT_TGT_REQ_RESP_ERR => {
                        // Update the stats.
                        if cur_msg
                            .flags
                            .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                        {
                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                let was_healthy = target_server.stats.is_healthy();

                                // This is for the "controlled" latency test.
                                // We do not want that failure to mix with the user
                                // traffic stats so call report_req_failed_internal
                                // instead.
                                target_server.stats.handle_req_failed_internal(
                                    cur_msg.timestamp,
                                    cur_msg.para8[1],
                                );

                                // A bad latency report on a healthy target_server could affect
                                // the selection of the target server.
                                if was_healthy {
                                    Self::update_selection_vectors(input_ports, &cur_msg);
                                }
                            }
                        } else {
                            // An error in the response for the user traffic.
                            if let Some(stats) = crate::NetworkMonitor::get_mut_all_servers_stats(
                                input_ports,
                                &cur_msg,
                            ) {
                                stats.handle_resp_err(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                    cur_msg.para8[1],
                                );
                            }

                            if let Some(target_server) =
                                NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                            {
                                target_server.stats.handle_resp_err(
                                    cur_msg.timestamp,
                                    cur_msg.para8[0],
                                    cur_msg.para32[0],
                                    cur_msg.para32[1],
                                    cur_msg.para8[1],
                                );
                                // User traffic should not select that target again.
                                // So always refresh the selection_vectors on every user
                                // traffic error.
                                Self::update_selection_vectors(input_ports, &cur_msg);
                            }
                        }
                    }
                    EVENT_REPORT_TGT_SEND_FAILED => {
                        // An error just sending a request.
                        if let Some(target_server) =
                            NetworkMonitor::get_mut_target_server(input_ports, &cur_msg)
                        {
                            let was_healthy = target_server.stats.is_healthy();

                            target_server.stats.handle_send_failed(
                                cur_msg.timestamp,
                                cur_msg.para8[1],
                                cur_msg.para16[0],
                            );

                            let update_selection_vectors = if cur_msg
                                .flags
                                .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                            {
                                was_healthy
                            } else {
                                true
                            };

                            if update_selection_vectors {
                                Self::update_selection_vectors(input_ports, &cur_msg);
                            }
                        }
                    }
                    EVENT_REPORT_REQ_FAILED => {
                        // Having no server available on startup is "normal". Ignore these for
                        // first 15 seconds uptime of this task.
                        if !(cur_msg.para8[1] == REQUEST_FAILED_NO_SERVER_AVAILABLE
                            && self.init_time.elapsed() < Duration::from_secs(15))
                        {
                            // Update the stats. Not related to a specific target server
                            // so update only the all_servers stats.
                            if let Some(stats) = crate::NetworkMonitor::get_mut_all_servers_stats(
                                input_ports,
                                &cur_msg,
                            ) {
                                if cur_msg
                                    .flags
                                    .intersects(NetmonFlags::HEADER_SBSD_SERVER_HC_SET)
                                {
                                    stats.handle_req_failed_internal(
                                        cur_msg.timestamp,
                                        cur_msg.para8[1],
                                    );
                                } else {
                                    stats.handle_req_failed(cur_msg.timestamp, cur_msg.para8[1]);
                                }
                            }
                        }
                    }
                    _ => {
                        log::error!(
                            "process_mut_globals unexpected event id {}",
                            cur_msg.event_id
                        );
                        // Do nothing. Consume the bad message.
                    }
                }

                // Check if more messages are available.
                match self.netmon_rx.try_recv() {
                    Ok(next_msg) => {
                        cur_msg = next_msg;
                    }
                    Err(_e) => {
                        // No more messages.
                        return None;
                    }
                }

                if !cur_msg
                    .flags
                    .intersects(NetmonFlags::NEED_GLOBAL_WRITE_MUTEX)
                {
                    // Does not requires a global mutex.
                    // Do not consume that message here.
                    return Some(cur_msg);
                }
            }
        }
    }
*/
