use std::path::Path;

use common::{basic_types::*, log_safe};

use crate::shared_types::{GlobalsWorkdirConfigMT, GlobalsWorkdirStatusMT};

use anyhow::{anyhow, Result};
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

// Design
//
// Up to 25x20MB files are stored in ~/suibase/workdirs/common/autocoins/data
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
//    - Open for read the file ~/suibase/workdirs/common/autocoins/data/{user.keypair public key in base58}/{file_nb}.data
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
    globals_mainnet_config: GlobalsWorkdirConfigMT,
    globals_devnet_status: GlobalsWorkdirStatusMT,
    globals_testnet_status: GlobalsWorkdirStatusMT,
    globals_mainnet_status: GlobalsWorkdirStatusMT,
    acoinsmon_rx: ACoinsMonRx,
    user_keypair: Option<LocalUserKeyPair>,
    acoins_client: Option<ACoinsClient>,
    mode: ServerMode,
}

impl ACoinsMonitor {
    pub fn new(
        globals_devnet_config: GlobalsWorkdirConfigMT,
        globals_testnet_config: GlobalsWorkdirConfigMT,
        globals_devnet_status: GlobalsWorkdirStatusMT,
        globals_testnet_status: GlobalsWorkdirStatusMT,
        globals_mainnet_config: GlobalsWorkdirConfigMT,
        globals_mainnet_status: GlobalsWorkdirStatusMT,
        acoinsmon_rx: ACoinsMonRx,
        mode: ServerMode,
    ) -> Self {
        Self {
            globals_devnet_config,
            globals_testnet_config,
            globals_devnet_status,
            globals_testnet_status,
            globals_mainnet_config,
            globals_mainnet_status,

            acoinsmon_rx,
            user_keypair: None,
            acoins_client: None,

            mode,
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

    async fn audit(&mut self) {
        // If autocoins is NOT enabled AND there is no autocoins directory, then just do nothing.
        let (tstarted, tenabled, tsui_address, tmode) = {
            let globals_read_guard = self.globals_testnet_config.read().await;
            let globals = &*globals_read_guard;
            let user_config = &globals.user_config;
            (
                user_config.is_user_request_start(),
                user_config.is_autocoins_enabled(),
                user_config.autocoins_address(),
                user_config.autocoins_mode(),
            )
        };

        let (dstarted, denabled, dsui_address, dmode) = {
            let globals_read_guard = self.globals_devnet_config.read().await;
            let globals = &*globals_read_guard;
            let user_config = &globals.user_config;
            (
                user_config.is_user_request_start(),
                user_config.is_autocoins_enabled(),
                user_config.autocoins_address(),
                user_config.autocoins_mode(),
            )
        };

        let (mstarted, menabled, msui_address, mmode) = {
            let globals_read_guard = self.globals_mainnet_config.read().await;
            let globals = &*globals_read_guard;
            let user_config = &globals.user_config;
            (
                user_config.is_user_request_start(),
                user_config.is_autocoins_enabled(),
                user_config.autocoins_address(),
                user_config.autocoins_mode(),
            )
        };

        let path = common::shared_types::get_workdir_common_path().join("autocoins");
        if !path.exists() {
            if (!tstarted && !tenabled) && (!dstarted && !denabled) && (!mstarted && !menabled) {
                return; // Don't even touch the FS if the user never enabled autocoins.
            }
            if let Err(error) = tokio::fs::create_dir_all(&path).await {
                let err_msg = format!("failed to create directory {}: {}", path.display(), error);
                log_safe!(err_msg);
                return;
            }
        }

        // Each installation has a user.keypair file created in ~/suibase/workdirs/common/autocoins
        //
        // Make sure a user.keypair file already exists or is created. If not successful, then just
        // do not bother to run the protocol because the FS is somehow not readable/accessible.
        let user_keypair_file = path.join("user.keypair");
        let user_keypair = match LocalUserKeyPair::from_file(user_keypair_file).await {
            Ok(user_keypair) => user_keypair,
            Err(error) => {
                let err_msg = format!("{}", error);
                log_safe!(err_msg);
                return;
            }
        };

        // Run the client side of the POI protocol.
        if self.acoins_client.is_none() {
            let acoins_client = ACoinsClient::new(self.mode);
            self.acoins_client = Some(acoins_client);
        }
        if let Some(acoins_client) = &mut self.acoins_client {
            let _ = acoins_client
                .run_poi(
                    path,
                    user_keypair.get_kp(),
                    tstarted,
                    tenabled,
                    &tsui_address,
                    tmode,
                    dstarted,
                    denabled,
                    &dsui_address,
                    dmode,
                    mstarted,
                    menabled,
                    &msui_address,
                    mmode,
                )
                .await;
        }
    }

    async fn process_msg(&mut self, msg: ACoinsMonMsg) {
        {
            match msg.event_id {
                EVENT_AUDIT => {
                    self.audit().await;
                }
                _ => {
                    log::debug!("process_msg unexpected event id {}", msg.event_id);
                }
            }
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
