use anyhow::Result;
use chrono::{Local, Utc};
use std::path::Path;

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
//     succeed, but are significanly more likely to be rate limited by the server.
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

use crate::log_safe_err;

use super::{StatusState, StatusYaml, UserKeypair, ACOINS_PROTOCOL_VERSION_LATEST};
pub struct ACoinsClient {
    client: Client,
}

impl ACoinsClient {
    /// Create a new ACoinsClient.
    pub fn new() -> Self {
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
        }
    }

    async fn run_network_protocol(
        &mut self,
        status_yaml: &mut StatusYaml,
        user_keypair: &UserKeypair,
    ) -> Result<()> {
        // Run the login+verify+download protocol (see integration_tests.rs: test_verification() for example).
        Ok(())
    }

    pub async fn run_poi<P: AsRef<Path>>(
        &mut self,
        autocoins_dir: P, // Path to the ~/suibase/workdirs/common/autocoins directory.
        user_keypair: &UserKeypair,
        testnet_started: bool, // User controlled. true when testnet services are started.
        testnet_enabled: bool, // User controlled. true when suibase.yaml 'autocoins_enabled: true' for testnet.
        testnet_address: Option<String>, // This is the user specified deposit address in suibase.yaml.
        devnet_started: bool,
        devnet_enabled: bool,
        devnet_address: Option<String>,
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

        status.tenabled = testnet_enabled;
        status.denabled = devnet_enabled;
        status.tstarted = testnet_started;
        status.dstarted = devnet_started;

        if (status.tenabled && status.tstarted) || (status.denabled && status.dstarted) {
            let now = Utc::now();

            #[cfg(any(test, feature = "integration-tests"))]
            {
                
            }
            // Do not attempt to run the protocol if last attempt was less than 1 hour ago.
            status.last_verification_attempt = Some(now);
            match self.run_network_protocol(&mut status, &user_keypair).await {
                Ok(_) => {
                    let now = Utc::now();
                    status.last_verification_ok = Some(now);
                }
                Err(e) => {
                    let now = Utc::now();
                    status.last_verification_failed = Some(now);

                    let err_msg = format!("Failed to run network protocol: {}", e);
                    crate::log_safe_err!(err_msg);
                }
            }
        }

        // Update percent_downloaded.
        let storage_dir = autocoins_dir.as_ref().join("data");

        // If status.yaml deposit_address is not matching the configuration, then
        // force the status to the configured address and show a deposit of 0.
        //
        // The server also independently reset the deposit total to zero whenever
        // detecting a change of address.
        //
        // Note: On initialization, it might be possible that the deposit address
        //       is not yet configured. This is fine, the server will just
        //       start to run the POI protocol and skip deposit until known.
        //
        if let Some(testnet_address) = testnet_address.as_ref() {
            if status.tsui_address.as_ref() != Some(testnet_address) {
                status.tsui_address = Some(testnet_address.clone());
                status.tsui_deposit = 0;
            }
        }

        if let Some(devnet_address) = devnet_address.as_ref() {
            if status.dsui_address.as_ref() != Some(devnet_address) {
                status.dsui_address = Some(devnet_address.clone());
                status.dsui_deposit = 0;
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
