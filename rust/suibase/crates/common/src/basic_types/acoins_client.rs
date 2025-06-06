use anyhow::{anyhow, Result};
use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::Utc;
use futures::StreamExt;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncWriteExt;

// Run autocoins proof-of-installation protocol for the client.
//
// Periodic calls to ACoinsClient::run_poi() are expected every ACoinsClient::CLIENT_EXECUTION_INTERVAL.
//
// run_poi() performs the client side of the protocols describe in src/api/def_methods in the poi-server crate.
//
// This object is the sole writer of ~/suibase/workdirs/common/autocoins/status.yaml
//
// status.yaml format is:
// ========================
// percent_downloaded: 100  <- From 0 to 100
// last_verification_attempt: 79836235 <- Unix timestamp in seconds
// last_verification_ok: 79836235 <- Unix timestamp in seconds
// last_verification_failed: 79830123 <- Unix timestamp in seconds
// day_offset: 283712 <- Offset in seconds during the day
//
// tstatus: "OK", "VERIFYING", "DOWNLOADING", "DOWN"
// tenabled: true, false
// tstopped: true, false
// tlast_error: "testnet error message"
// tlast_warning: "testnet warning message"
// tsui_address: "0x1234cccc...1726374abcd" <- 64 chars, but shorten for display to '(0x1234..abcd)'
// tsui_deposit: 5   <- Displayed as ">999" if greater than 999
// twal_deposit: 0   <- Displayed as ">999" if greater than 999
//
// dstatus: "OK", "VERIFYING", "DOWNLOADING", "DOWN"
// denabled: true, false
// dstopped: true, false
// dlast_error: "devnet error message"
// dlast_warning: "devnet warning message"
// dsui_address: "0x1234cccc...1726374abcd" <- 64 chars, but shorten for display to '(0x1234..abcd)'
// dsui_deposit: 5   <- Displayed as ">999" if greater than 999
//
// ========================
//   (Note: Use acoins_status_yaml::StatusYaml for loading/modifying/saving the status.yaml).
//
// last_verification_* timestamps are to control the rate at which the client holdoff
// its verification attempts:
//   - No more than one verification_ok per day is necessary.
//   - No more than 4 retries on verification_failed per day is allowed.
//   - verification attempts are preffered to be done within a window one hour
//     after and before the day_offset. Attempts outside that 22 windows *may*
//     succeed, but are significanly more likely to be rejected by the server.
//
// day_offset is encoded in the challenge provided in the LoginResponse. It is also saved
// in the status.yaml for aligning future verification attempts.
//
// The deposit address and total are encoded in the VerifyResponse (when pass) and are also
// saved in the status.yaml.
//
// The storage is in sub-directory:
//       ~/suibase/workdirs/common/autocoins/data/<pk>.<file_number>
//
// percent_downloaded is calculated by counting the number of <pk>.<file_number> files
// for current "pk" and divide by basic_types::ACOINS_STORAGE_NB_FILES
//
// The current "pk" is the public key read with UserKeypair::from_file() on:
//       ~/suibase/workdirs/common/autocoins/user.keystore
//
// Status.taml status field:
//  OK --> Everything is ready and deposit are done regularly. percent_download is 100%.
//
//  DOWNLOADING --> Data is downloading for proof-of-installation. Deposit not done regularly.
//
//  VERIFYING --> Data 100% downloaded, but verification is ongoing. Deposit not done regularly.
//
//  DOWN --> No internet connectivity or any other preventing a successful verification since
//           more than 30 days.
//
// Note: Shell script adds "NOT RUNNING" and "STOPPED" status overriding the status.yaml
//
// enabled/stopped are in status.yaml only for debugging purpose. They reflect the last user
// configuration processed by the suibase-daemon. These status should eventually synch (within
// a few seconds) with a user configuration change (e.g. suibase.yaml edit).

use reqwest::Client;

use crate::{
    basic_types::{
        ACoinsChallenge, ACoinsVerifyBuffer, ACOINS_SERVER_STAGE_API_PORT,
        ACOINS_SERVER_STAGE_DOWNLOAD_PORT, ACOINS_SERVER_TEST_API_PORT,
        ACOINS_SERVER_TEST_DOWNLOAD_PORT, ACOINS_STORAGE_FILE_SIZE,
    },
    log_safe, log_safe_err, log_safe_warn,
};

use super::{
    parse_json_rpc_response, ClientMode, LoginResponse, ServerMode, StatusYaml, UserKeypair,
    VerifyResponse, ACOINS_PROTOCOL_VERSION_LATEST, ACOINS_SERVER_PUBLIC_API_PORT,
    ACOINS_SERVER_PUBLIC_DOWNLOAD_PORT, ACOINS_STORAGE_NB_FILES,
};
pub struct ACoinsClient {
    client: Client,
    last_run_network_protocol: Option<chrono::DateTime<Utc>>,
    mode: ServerMode,
}

impl ACoinsClient {
    /// Create a new ACoinsClient.
    pub fn new(mode: ServerMode) -> Self {
        ACoinsClient {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .user_agent(format!(
                    "client/{}/{}",
                    env!("CARGO_PKG_VERSION"),
                    ACOINS_PROTOCOL_VERSION_LATEST
                ))
                .build()
                .unwrap_or_default(),
            last_run_network_protocol: None,
            mode,
        }
    }

    pub fn is_test_setup(&self) -> bool {
        self.mode == ServerMode::Test
    }

    pub fn is_stage_setup(&self) -> bool {
        self.mode == ServerMode::Stage
    }

    async fn request_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = if self.is_test_setup() {
            format!("http://127.0.0.1:{}", ACOINS_SERVER_TEST_API_PORT)
        } else if self.is_stage_setup() {
            format!("http://127.0.0.1:{}", ACOINS_SERVER_STAGE_API_PORT)
        } else {
            format!(
                "https://poi-server.suibase.io:{}",
                ACOINS_SERVER_PUBLIC_API_PORT
            )
        };

        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });

        self.client
            .post(url)
            .json(&request_body)
            .timeout(Duration::from_secs(60))
            .send()
            .await
    }

    async fn request_download_stream(
        &self,
        download_id: &str,
        pk_short: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let url = if self.is_test_setup() {
            format!("http://127.0.0.1:{}", ACOINS_SERVER_TEST_DOWNLOAD_PORT)
        } else if self.is_stage_setup() {
            format!("http://127.0.0.1:{}", ACOINS_SERVER_STAGE_DOWNLOAD_PORT)
        } else {
            format!(
                "https://poi-server.suibase.io:{}",
                ACOINS_SERVER_PUBLIC_DOWNLOAD_PORT
            )
        };

        self.client
            .get(format!("{}/download/{}/{}", url, download_id, pk_short))
            .timeout(Duration::from_secs(60))
            .send()
            .await
    }

    async fn run_protocol_outer<P: AsRef<Path>>(
        &mut self,
        autocoins_dir: P, // Path to the ~/suibase/workdirs/common/autocoins directory.
        status_yaml: &mut StatusYaml,
        user_keypair: &UserKeypair,
        cfg_testnet_address: &Option<String>,
        cfg_devnet_address: &Option<String>,
        cfg_mainnet_address: &Option<String>,
    ) -> Result<()> {
        let dt_now = Utc::now();

        let is_production = !self.is_test_setup() && !self.is_stage_setup();
        if is_production {
            // Rate limit the protocol to 1 per hour using the storage persistent state.
            if let Some(last_verification_attempt) = status_yaml.last_verification_attempt {
                let elapsed = dt_now.signed_duration_since(last_verification_attempt);
                if elapsed.num_hours() < 1 {
                    return Ok(());
                }
            }
            // Rate limit also using the in-memory state.
            if let Some(last_run_network_protocol) = self.last_run_network_protocol {
                let elapsed = dt_now.signed_duration_since(last_run_network_protocol);
                if elapsed.num_hours() < 1 {
                    return Ok(());
                }
            }

            // Do not attempt execution in the "maintenance window" of the server.
            let now = dt_now.timestamp() % 86400;
            if !(360..=82800).contains(&now) {
                return Ok(());
            }
        }

        let mut run_protocol = false;
        if !is_production {
            run_protocol = true;
        } else {
            // Always run the protocol once after the process is started.
            if self.last_run_network_protocol.is_none() {
                run_protocol = true;
            } else {
                // TODO Do also in-memory check of this (do not trust solely the status_yaml)

                // If there was no successful verification in the last 4 hours, allow to
                // run the protocol again.
                if let Some(last_verification_failed) = status_yaml.last_verification_failed {
                    let elapsed = dt_now.signed_duration_since(last_verification_failed);
                    if elapsed.num_hours() > 4 {
                        run_protocol = true;
                    }
                }

                // if a verification was successful in the last 24 hours then run the protocol
                // again.
                // TODO: Try to align on the day_offset if known.
                if let Some(last_verification_ok) = status_yaml.last_verification_ok {
                    // Run the protocol if it has been more than 24 hours ago according to state_yaml.
                    let elapsed = dt_now.signed_duration_since(last_verification_ok);
                    if elapsed.num_hours() > 24 {
                        run_protocol = true;
                    }
                }
            }
        };

        // Always re-run the protocol if it has been more than 25 hours since the last
        // time the protocol was run. This is a simplified safeguard in case there is a
        // bug in the above synchronization logic.
        if let Some(last_run_network_protocol) = self.last_run_network_protocol {
            let elapsed = dt_now.signed_duration_since(last_run_network_protocol);
            if elapsed.num_hours() > 25 {
                run_protocol = true;
            }
        } else {
            run_protocol = true;
        }

        if !run_protocol {
            return Ok(());
        }

        self.last_run_network_protocol = Some(dt_now);
        let now = Utc::now();
        status_yaml.last_verification_attempt = Some(now);
        match self
            .run_protocol_inner(
                &autocoins_dir,
                status_yaml,
                user_keypair,
                cfg_testnet_address,
                cfg_devnet_address,
                cfg_mainnet_address,
            )
            .await
        {
            Ok(_) => {
                let now = Utc::now();
                status_yaml.last_verification_ok = Some(now);
            }
            Err(e) => {
                let now = Utc::now();
                status_yaml.last_verification_failed = Some(now);

                let err_msg = format!("Failed to run network protocol: {}", e);
                crate::log_safe_err!(err_msg);
            }
        }

        Ok(())
    }

    async fn run_protocol_inner<P: AsRef<Path>>(
        &mut self,
        autocoins_dir: P,
        status_yaml: &mut StatusYaml,
        user_keypair: &UserKeypair,
        cfg_testnet_address: &Option<String>,
        cfg_devnet_address: &Option<String>,
        cfg_mainnet_address: &Option<String>,
    ) -> Result<()> {
        log_safe!(format!(
            "Running ACoinsClient protocol for user keypair: {}",
            user_keypair.pk_to_string()
        ));
        // Do the login step of the protocol.
        let pk = user_keypair.pk_to_string();
        let mut signer = match self.do_login_step(&pk).await {
            Ok(signer) => signer,
            Err(e) => {
                let err_msg = format!("Failed to login: {}", e);
                return Err(anyhow!(err_msg));
            }
        };

        // Try to get the challenge bytes requested from the storage.
        let (data_str, req_file, file_count) = self
            .try_read_from_storage(&autocoins_dir, &pk, &signer)
            .await;

        status_yaml.percent_downloaded =
            ((file_count * 100) / ACOINS_STORAGE_NB_FILES as usize) as u8;

        // Do the verify step of the protocol.
        let verify_response = match self
            .do_verify_step(
                &pk,
                &mut signer,
                user_keypair,
                &data_str,
                &req_file,
                cfg_testnet_address,
                cfg_devnet_address,
                cfg_mainnet_address,
            )
            .await
        {
            Ok(verify_response) => verify_response,
            Err(e) => {
                let err_msg = format!("Failed to verify: {}", e);
                return Err(anyhow!(err_msg));
            }
        };

        // Update status.yaml using server side info provided in the VerifyResponse.
        if verify_response.tsui_address.is_some()
            && (status_yaml.tsui_address != verify_response.tsui_address)
        {
            status_yaml.tsui_address = verify_response.tsui_address.clone();
        }
        if verify_response.tsui_deposit.is_some()
            && (status_yaml.tsui_deposit != verify_response.tsui_deposit)
        {
            status_yaml.tsui_deposit = verify_response.tsui_deposit;
        }
        if verify_response.twal_deposit.is_some()
            && (status_yaml.twal_deposit != verify_response.twal_deposit)
        {
            status_yaml.twal_deposit = verify_response.twal_deposit;
        }

        if verify_response.dsui_address.is_some()
            && (status_yaml.dsui_address != verify_response.dsui_address)
        {
            status_yaml.dsui_address = verify_response.dsui_address.clone();
        }
        if verify_response.dsui_deposit.is_some()
            && (status_yaml.dsui_deposit != verify_response.dsui_deposit)
        {
            status_yaml.dsui_deposit = verify_response.dsui_deposit;
        }
        if verify_response.dwal_deposit.is_some()
            && (status_yaml.dwal_deposit != verify_response.dwal_deposit)
        {
            status_yaml.dwal_deposit = verify_response.dwal_deposit;
        }

        if verify_response.msui_address.is_some()
            && (status_yaml.msui_address != verify_response.msui_address)
        {
            status_yaml.msui_address = verify_response.msui_address.clone();
        }
        if verify_response.msui_deposit.is_some()
            && (status_yaml.msui_deposit != verify_response.msui_deposit)
        {
            status_yaml.msui_deposit = verify_response.msui_deposit;
        }
        if verify_response.mwal_deposit.is_some()
            && (status_yaml.mwal_deposit != verify_response.mwal_deposit)
        {
            status_yaml.mwal_deposit = verify_response.mwal_deposit;
        }

        // Proceed with a download authorized by the server. Catch and log errors, but
        // let the protocol continue successfully. If failed download persist, then the
        // user will notice download never reaching "100%".
        if self
            .do_download_step(&autocoins_dir, &pk, &verify_response)
            .await
        {
            // A file download was successful, so clean-up to avoid unexpected storage
            // accumulation (and also udpate percent_downloaded)
            self.storage_maintenance(&autocoins_dir, &pk, status_yaml)
                .await;
        }

        Ok(())
    }

    async fn storage_maintenance<P: AsRef<Path>>(
        &self,
        autocoins_dir: P,
        pk: &str,
        status_yaml: &mut StatusYaml,
    ) {
        // Do storage cleanup to avoid filling up the disk (in case of bug!? or malicious user!?).

        // Remove one file with unexpected name (only one at most per call).
        // Expected name matches the pattern "{pk}.*".
        //
        // Also, update the status_yaml.percent_downloaded.
        //
        // Count the number of files matching the pattern "{pk}.*"
        // where '*' can be from 1 to ACOINS_STORAGE_NB_FILES.
        let data_dir = autocoins_dir.as_ref().join("data");

        // Read all entries from directory
        let Ok(mut entries) = tokio::fs::read_dir(&data_dir).await else {
            log_safe_err!(format!("Failed to read directory: {}", data_dir.display()));
            return;
        };

        let mut file_to_remove: Option<PathBuf> = None;
        let mut n_file_valid = 0;

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(file_name) = entry.file_name().into_string() {
                // Check if file follows the expected pattern "{pk}.*"
                let prefix = format!("{}.", pk);
                if !file_name.starts_with(&prefix) {
                    // This file doesn't match our expected prefix, remove it
                    if file_to_remove.is_none() {
                        if let Ok(file_path) = entry.path().canonicalize() {
                            file_to_remove = Some(file_path);
                        }
                    }
                } else {
                    // This file matches our expected pattern, count it if the file number is also valid.
                    let mut file_number_valid = false;
                    if let Ok(num_str) = file_name[prefix.len()..].parse::<usize>() {
                        if num_str >= 1 && num_str <= ACOINS_STORAGE_NB_FILES as usize {
                            n_file_valid += 1;
                            file_number_valid = true;
                        }
                    }
                    if !file_number_valid && file_to_remove.is_none() {
                        if let Ok(file_path) = entry.path().canonicalize() {
                            file_to_remove = Some(file_path);
                        }
                    }
                }
            }
        }

        if let Some(file_path) = file_to_remove {
            // Double-check that we're only deleting files in the data directory
            if let Ok(data_dir_canonical) = data_dir.canonicalize() {
                if file_path.starts_with(&data_dir_canonical) {
                    if let Err(e) = tokio::fs::remove_file(&file_path).await {
                        log_safe_err!(format!(
                            "Failed to remove unexpected file {}: {}",
                            file_path.display(),
                            e
                        ));
                    } else {
                        // It could be normal to remove files if, say, the user keypair was changed.
                        log_safe_warn!(format!("Removed unexpected file {}", file_path.display()));
                    }
                } else {
                    log_safe_err!(format!(
                        "Failed to delete file because canonicalize path differences: {} and {}",
                        data_dir_canonical.display(),
                        file_path.display()
                    ));
                }
            }
        }

        // Update the status_yaml.percent_downloaded
        status_yaml.percent_downloaded =
            ((n_file_valid * 100) / ACOINS_STORAGE_NB_FILES as usize) as u8;
    }

    async fn do_download_step<P: AsRef<Path>>(
        &mut self,
        autocoins_dir: P,
        pk: &String,
        verify_response: &VerifyResponse,
    ) -> bool {
        let Some(download_id) = verify_response.download_id.as_ref() else {
            return false;
        };

        let Some(file_number) = verify_response.download_fn.as_ref() else {
            return false;
        };

        let pk_short = if pk.len() >= 8 {
            pk.split_at(8).0
        } else {
            return false;
        };

        // Delete the file if it already exists. If the server asks to
        // download an existing files, then it means that the file is either
        // corrupted or somehow no longer valid with the latest POI protocol.
        let data_dir = autocoins_dir.as_ref().join("data");
        let file_path = data_dir.join(format!("{}.{}", pk, file_number));
        if file_path.exists() {
            if let Err(e) = tokio::fs::remove_file(&file_path).await {
                log_safe_err!(format!(
                    "Failed to remove existing file {}: {}",
                    file_path.display(),
                    e
                ));
                return false;
            }
        }

        if let Err(e) = tokio::fs::create_dir_all(&data_dir).await {
            log_safe_err!(format!(
                "Failed to create directory {}: {}",
                data_dir.display(),
                e
            ));
            return false;
        }

        // Download the file from the server as a stream so we
        // can protect against unexpectedly large files (hack?).
        let response = match self.request_download_stream(download_id, pk_short).await {
            Ok(response) => response,
            Err(e) => {
                log_safe_err!(format!(
                    "Failed to initiate download of file {}: {}",
                    file_number, e
                ));
                return false;
            }
        };

        if response.status().is_server_error() {
            log_safe_err!(format!(
                "Server error while downloading file {}: {}",
                file_number,
                response.status()
            ));
            return false;
        }

        if response.status().is_client_error() {
            log_safe_err!(format!(
                "Client error while downloading file {}: {}",
                file_number,
                response.status()
            ));
            return false;
        }

        if response.status() != reqwest::StatusCode::OK {
            log_safe_err!(format!(
                "Failed to download file {}: {}",
                file_number,
                response.status()
            ));
            return false;
        }

        // Check if size is out of specs.
        const MIN_DOWNLOAD_SIZE: u64 = ACOINS_STORAGE_FILE_SIZE as u64;
        const MAX_DOWNLOAD_SIZE: u64 = MIN_DOWNLOAD_SIZE + 8192;

        if let Some(content_length) = response.content_length() {
            if content_length > MAX_DOWNLOAD_SIZE {
                log_safe_err!(format!(
                    "File size too large: {} bytes (max: {} bytes)",
                    content_length, MAX_DOWNLOAD_SIZE
                ));
                return false;
            }
            if content_length < MIN_DOWNLOAD_SIZE {
                log_safe_err!(format!(
                    "File size too small: {} bytes (min: {} bytes)",
                    content_length, MIN_DOWNLOAD_SIZE
                ));
                return false;
            }
        } else {
            log_safe_err!("Failed to get content length from response");
            return false;
        }

        // Use streaming to avoid loading the entire file into memory at once.
        // From this point, the file will be deleted on any error.
        let mut file_error = false;
        match tokio::fs::File::create(&file_path).await {
            Ok(mut file) => {
                let mut stream = response.bytes_stream();
                let mut downloaded: u64 = 0;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            downloaded += chunk.len() as u64;
                            if downloaded > MAX_DOWNLOAD_SIZE {
                                log_safe_err!("Download exceeded maximum allowed size");
                                file_error = true;
                                break;
                            }
                            if let Err(e) = file.write_all(&chunk).await {
                                log_safe_err!(format!("Failed to write chunk: {}", e));
                                file_error = true;
                                break;
                            }
                        }
                        Err(e) => {
                            log_safe_err!(format!("Error downloading chunk: {}", e));
                            file_error = true;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                log_safe_err!(format!(
                    "Failed to create file '{}': {}",
                    file_path.to_string_lossy(),
                    e
                ));
                file_error = true;
            }
        }

        if file_error {
            if file_path.exists() {
                if let Err(e) = tokio::fs::remove_file(&file_path).await {
                    log_safe_err!(format!(
                        "Failed to remove file {}: {}",
                        file_path.display(),
                        e
                    ));
                }
            }
            return false;
        }

        true // Successfully downloaded a new file.
    }

    async fn do_login_step(&mut self, pk: &str) -> Result<ACoinsVerifyBuffer> {
        let params = serde_json::json!({"pk": pk});

        let response = self
            .request_rpc("login", params)
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        let login_response = parse_json_rpc_response::<LoginResponse>(response).await?;

        let challenge_str = login_response
            .challenge
            .ok_or_else(|| anyhow::anyhow!("Challenge not found in login response"))?;

        let mut signer = ACoinsVerifyBuffer::new();
        signer
            .set_challenge_from_base64(challenge_str)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        signer
            .set_pk_from_base64(pk)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(signer)
    }

    async fn try_read_from_storage<P: AsRef<Path>>(
        &self,
        autocoins_dir: P,
        pk: &str,
        signer: &ACoinsVerifyBuffer,
    ) -> (Option<String>, Option<usize>, usize) {
        // Attempt to return the read data and a file_number recommended to be downloaded next.
        // On any error, the recommended file is the one that was requested in the challenge.
        let acoins_challenge = ACoinsChallenge::from_bytes(signer.challenge());
        let file_number = acoins_challenge.file_number() as usize;
        let file_offset = acoins_challenge.file_offset() as usize;
        let length = acoins_challenge.length() as usize;

        let data_dir = autocoins_dir.as_ref().join("data");
        let file_path = data_dir.join(format!("{}.{}", pk, file_number));

        let (recommended_file, file_count) = self.find_missing_file_number(&data_dir, pk).await;

        if !file_path.exists() {
            return (None, Some(file_number), file_count);
        }

        let read_result = async {
            let mut file = tokio::fs::File::open(&file_path).await?;

            // Seek to the correct offset
            tokio::io::AsyncSeekExt::seek(&mut file, std::io::SeekFrom::Start(file_offset as u64))
                .await?;

            // Read exactly 'length' bytes
            let mut data = vec![0u8; length];
            tokio::io::AsyncReadExt::read_exact(&mut file, &mut data).await?;

            Ok::<Vec<u8>, std::io::Error>(data)
        }
        .await;

        // Process read result
        match read_result {
            Ok(data) => {
                // Successfully read the data
                let encoded_data = Base64UrlUnpadded::encode_string(&data);
                (Some(encoded_data), recommended_file, file_count)
            }
            // Error reading the challenge file (corrupted?), suggest to re-download it.
            Err(_) => (None, Some(file_number), file_count),
        }
    }

    async fn find_missing_file_number<P: AsRef<Path>>(
        &self,
        data_dir: P,
        pk: &str,
    ) -> (Option<usize>, usize) {
        // Returns both a missing file (if any) and the total number of files
        // already downloaded.
        static_assertions::const_assert!(ACOINS_STORAGE_NB_FILES <= 64);
        let max_nb_files = ACOINS_STORAGE_NB_FILES as usize;

        // Read all entries from directory
        let mut entries = match tokio::fs::read_dir(data_dir.as_ref()).await {
            Ok(entries) => entries,
            Err(_) => return (Some(1), 0), // If we can't read the directory, suggest file #1
        };

        let mut file_bitmap: u64 = 0;
        let mut file_count = 0;

        // Process each directory entry
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(file_name) = entry.file_name().into_string() {
                // Extract file number from the name pattern "pk.number"
                let prefix = format!("{}.", pk);
                if file_name.starts_with(&prefix) {
                    if let Ok(num) = file_name[prefix.len()..].parse::<usize>() {
                        // Only set the bit if the file number is within the valid range
                        if num >= 1 && num <= max_nb_files {
                            file_bitmap |= 1u64 << (num - 1); // Set the bit at position (num-1)
                            file_count += 1;
                        }
                    }
                }
            }
        }

        // Quick check if all files exist (all bits are set)
        let all_files_mask = (1u64 << max_nb_files) - 1;
        if file_bitmap == all_files_mask {
            return (None, max_nb_files); // All files exist
        }

        // Find the first missing file number by checking for the first unset bit
        for i in 1..=max_nb_files {
            if (file_bitmap & (1u64 << (i - 1))) == 0 {
                return (Some(i), file_count);
            }
        }

        // That should logically never happen... but just in case.
        (None, max_nb_files)
    }

    async fn do_verify_step(
        &mut self,
        pk: &str,
        signer: &mut ACoinsVerifyBuffer,
        user_keypair: &UserKeypair,
        data_str: &Option<String>,
        req_file: &Option<usize>,
        cfg_testnet_address: &Option<String>,
        cfg_devnet_address: &Option<String>,
        cfg_mainnet_address: &Option<String>,
    ) -> Result<VerifyResponse> {
        let mut params = serde_json::json!({
            "challenge": signer.challenge_str(),
            "pk": pk,
        });

        // Note: challenge and pk were already set in signer while parsing LoginResponse.
        if let Some(address) = cfg_testnet_address {
            params["testnet_address"] = serde_json::json!(address);
            let _ = signer.set_testnet_address_from_hex(address);
        }

        if let Some(address) = cfg_devnet_address {
            params["devnet_address"] = serde_json::json!(address);
            let _ = signer.set_devnet_address_from_hex(address);
        }

        if let Some(address) = cfg_mainnet_address {
            params["mainnet_address"] = serde_json::json!(address);
            let _ = signer.set_mainnet_address_from_hex(address);
        }

        if data_str.is_some() {
            let data_str_param = data_str.as_ref().unwrap();
            params["data"] = serde_json::json!(data_str_param);
            let _ = signer.set_challenge_response_from_base64(data_str.as_deref());
            if req_file.is_some() {
                let req_file_param = req_file.as_ref().unwrap();
                params["req_file"] = serde_json::json!(req_file_param);
                signer.set_req_file(*req_file_param as u8);
            }
        }

        let signature = signer.sign(user_keypair);
        params["signature"] = serde_json::json!(signature);

        let response = self
            .request_rpc("verify", params)
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        parse_json_rpc_response::<VerifyResponse>(response).await
    }

    pub async fn run_poi<P: AsRef<Path>>(
        &mut self,
        autocoins_dir: P, // Path to the ~/suibase/workdirs/common/autocoins directory.
        user_keypair: &UserKeypair,
        cfg_testnet_started: bool, // User controlled. true when testnet services are started.
        cfg_testnet_enabled: bool, // User controlled. true when suibase.yaml 'autocoins_enabled: true' for testnet.
        cfg_testnet_address: &Option<String>, // This is the user specified deposit address in suibase.yaml.
        cfg_testnet_mode: ClientMode,
        cfg_devnet_started: bool,
        cfg_devnet_enabled: bool,
        cfg_devnet_address: &Option<String>,
        cfg_devnet_mode: ClientMode,
        cfg_mainnet_enabled: bool,
        cfg_mainnet_started: bool,
        cfg_mainnet_address: &Option<String>,
        cfg_mainnet_mode: ClientMode,
    ) -> Result<()> {
        // Load status.yaml file (or start with default)
        let status_yaml = autocoins_dir.as_ref().join("status.yaml");
        let mut status = match StatusYaml::load(status_yaml).await {
            Ok(status) => status,
            Err(e) => {
                // Should never happen default if file does not exist.
                let err_msg = format!("Failed to load status.yaml: {}", e);
                log_safe_err!(err_msg);
                return Ok(());
            }
        };

        status.tenabled = cfg_testnet_enabled;
        status.denabled = cfg_devnet_enabled;
        status.tstarted = cfg_testnet_started;
        status.dstarted = cfg_devnet_started;
        status.menabled = cfg_mainnet_enabled;
        status.mstarted = cfg_mainnet_started;
        status.tmode = cfg_testnet_mode;
        status.dmode = cfg_devnet_mode;
        status.mmode = cfg_mainnet_mode;

        // For now, only testnet autocoins are really working, although the protocol already
        // supports mainnet for (may be) future deposits to loyal long-term users.
        if (status.tenabled && status.tstarted)
            || (status.denabled && status.dstarted)
            || (status.menabled && status.mstarted)
        {
            let _ = self
                .run_protocol_outer(
                    &autocoins_dir,
                    &mut status,
                    user_keypair,
                    cfg_testnet_address,
                    cfg_devnet_address,
                    cfg_mainnet_address,
                )
                .await;
        }

        let _storage_dir = autocoins_dir.as_ref().join("data");

        // Note: There are "cfg" addresses and "status" addresses.
        //
        // The "cfg" are the local configurations, the "status.yaml" are what the
        // remote server is using. They may be different for up to 24 hours
        // (the server will eventually match as the protocol is being run).

        // If the server side address is not matching the configuration, then
        // force the status to the configured one and show a deposit of 0.
        //
        // The server also independently reset the deposit total to zero whenever
        // detecting a change of address.
        //
        // Note: On initialization, it might be possible that the deposit address
        //       is not yet configured. This is fine, the server will just
        //       start to run the POI protocol and skip deposit until known.
        //
        if let Some(testnet_address) = cfg_testnet_address.as_ref() {
            if status.tsui_address.as_ref() != Some(testnet_address) {
                status.tsui_address = Some(testnet_address.clone());
                status.tsui_deposit = None;
            }
        }

        if let Some(devnet_address) = cfg_devnet_address.as_ref() {
            if status.dsui_address.as_ref() != Some(devnet_address) {
                status.dsui_address = Some(devnet_address.clone());
                status.dsui_deposit = None;
            }
        }

        if let Some(mainnet_address) = cfg_mainnet_address.as_ref() {
            if status.msui_address.as_ref() != Some(mainnet_address) {
                status.msui_address = Some(mainnet_address.clone());
                status.msui_deposit = None;
            }
        }

        // Always check to write back the status.yaml file (will be NOOP if nothing changed).
        if let Err(e) = status.save().await {
            let err_msg = format!("Failed to save status.yaml: {}", e);
            crate::log_safe_err!(err_msg);
        }

        Ok(())
    }
}
