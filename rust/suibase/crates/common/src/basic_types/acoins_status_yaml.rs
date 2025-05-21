use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
use tokio::fs;
use twox_hash::XxHash64;

use super::ClientMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum StatusState {
    Ok,
    #[default]
    Verifying,
    Downloading,
    Down,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusYaml {
    // Fields that are common to both testnet/devnet.
    #[serde(default)]
    pub percent_downloaded: u8,

    #[serde(
        default,
        with = "timestamp_format",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_verification_attempt: Option<DateTime<Utc>>,

    #[serde(
        default,
        with = "timestamp_format",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_verification_ok: Option<DateTime<Utc>>,

    #[serde(
        default,
        with = "timestamp_format",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_verification_failed: Option<DateTime<Utc>>,

    #[serde(default)]
    pub day_offset: u32,

    // Fields specific to testnet
    #[serde(default)]
    pub tstatus: StatusState,

    #[serde(default)]
    pub tenabled: bool,

    #[serde(default)]
    pub tstarted: bool,

    #[serde(default)]
    pub tmode: ClientMode,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tlast_error: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tlast_warning: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tsui_address: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tsui_deposit: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub twal_deposit: Option<u64>,

    // Fields specific to devnet
    #[serde(default)]
    pub dstatus: StatusState,

    #[serde(default)]
    pub denabled: bool,

    #[serde(default)]
    pub dstarted: bool,

    #[serde(default)]
    pub dmode: ClientMode,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dlast_error: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dlast_warning: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dsui_address: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dsui_deposit: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dwal_deposit: Option<u64>,

    // Fields specific to mainnet
    #[serde(default)]
    pub mstatus: StatusState,

    #[serde(default)]
    pub menabled: bool,

    #[serde(default)]
    pub mstarted: bool,

    #[serde(default)]
    pub mmode: ClientMode,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mlast_error: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mlast_warning: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msui_address: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msui_deposit: Option<u64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mwal_deposit: Option<u64>,

    // Fields not serialized to YAML
    #[serde(skip)]
    loaded_from: Option<PathBuf>,

    #[serde(skip)]
    content_hash: Option<u64>,
}

// Custom serialization for DateTime<Utc> to/from unix timestamps
mod timestamp_format {
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(date) => serializer.serialize_i64(date.timestamp()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp: Option<i64> = serde::Deserialize::deserialize(deserializer)?;
        match timestamp {
            Some(ts) => Ok(Some(Utc.timestamp_opt(ts, 0).unwrap())),
            None => Ok(None),
        }
    }
}

impl Default for StatusYaml {
    fn default() -> Self {
        Self {
            percent_downloaded: 0,
            last_verification_attempt: None,
            last_verification_ok: None,
            last_verification_failed: None,
            day_offset: 0,

            tstatus: StatusState::default(),
            tenabled: false,
            tstarted: false,
            tmode: ClientMode::default(),
            tlast_error: None,
            tlast_warning: None,
            tsui_address: None,
            tsui_deposit: None,
            twal_deposit: None,

            dstatus: StatusState::default(),
            denabled: false,
            dstarted: false,
            dmode: ClientMode::default(),
            dlast_error: None,
            dlast_warning: None,
            dsui_address: None,
            dsui_deposit: None,
            dwal_deposit: None,

            mstatus: StatusState::default(),
            menabled: false,
            mstarted: false,
            mmode: ClientMode::default(),
            mlast_error: None,
            mlast_warning: None,
            msui_address: None,
            msui_deposit: None,
            mwal_deposit: None,

            loaded_from: None,
            content_hash: None,
        }
    }
}

impl StatusYaml {
    /// Create a new StatusYaml with default values
    pub fn new() -> Self {
        Self::default()
    }

    // Calculate a content hash for change detection
    fn calculate_hash(&self) -> Result<u64> {
        // Create a temporary clone without metadata fields
        let mut clone = self.clone();
        clone.loaded_from = None;
        clone.content_hash = None;

        // Serialize and hash
        let yaml = serde_yaml::to_string(&clone)?;
        let mut hasher = XxHash64::default();
        yaml.hash(&mut hasher);
        Ok(hasher.finish())
    }

    /// Load from a YAML file, returns default if file doesn't exist
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        // Return default if file doesn't exist
        if !fs::try_exists(path).await? {
            let status = StatusYaml {
                loaded_from: Some(path.to_path_buf()),
                ..Default::default()
            };
            return Ok(status);
        }

        // Read file contents asynchronously
        let contents = fs::read_to_string(path).await?;
        let mut status: Self = serde_yaml::from_str(&contents)?;

        // Store the path and content hash
        status.loaded_from = Some(path.to_path_buf());
        status.content_hash = status.calculate_hash().ok();

        Ok(status)
    }

    /// Save to a YAML file
    pub async fn save(&mut self) -> Result<()> {
        // Do nothing if we don't have a path
        let path = match &self.loaded_from {
            Some(path) => path,
            None => return Ok(()),
        };

        // Check if content changed
        let current_hash = self.calculate_hash()?;
        if self.content_hash == Some(current_hash) {
            // No changes, skip saving
            return Ok(());
        }

        // Create parent directories in case they somehow have been
        // deleted since the load (!!!).
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Generate YAML content
        let yaml = serde_yaml::to_string(self)?;

        // Create a temporary file in the same directory
        let temp_path = path.with_extension("yaml.tmp");

        // Write content to temporary file first
        fs::write(&temp_path, &yaml).await?;

        // Atomically rename the temporary file to the target file
        fs::rename(temp_path, path).await?;

        // Update hash
        self.content_hash = Some(current_hash);

        Ok(())
    }
    /// Format deposit address for display
    pub fn format_deposit_address(&self) -> String {
        match &self.tsui_address {
            Some(addr) if addr.len() >= 10 => {
                let start = &addr[..6];
                let end = &addr[addr.len() - 4..];
                format!("({}..{})", start, end)
            }
            Some(addr) => addr.clone(),
            None => String::from(""),
        }
    }

    /// Format deposit total for display
    pub fn format_deposit_total(&self, deposit: Option<u64>) -> String {
        if let Some(deposit) = deposit {
            if deposit > 999 {
                ">999".to_string()
            } else {
                deposit.to_string()
            }
        } else {
            String::from("0")
        }
    }

    /// Update last verification attempt timestamp to now
    pub fn update_verification_attempt(&mut self) {
        self.last_verification_attempt = Some(Utc::now());
    }

    /// Update last successful verification timestamp to now
    pub fn update_verification_ok(&mut self) {
        let now = Utc::now();
        self.last_verification_attempt = Some(now);
        self.last_verification_ok = Some(now);
    }

    /// Update last failed verification timestamp to now
    pub fn update_verification_failed(&mut self) {
        let now = Utc::now();
        self.last_verification_attempt = Some(now);
        self.last_verification_failed = Some(now);
    }

    /// Get path to the status.yaml file
    pub fn get_status_path<P: AsRef<Path>>(autocoins_dir: P) -> PathBuf {
        autocoins_dir.as_ref().join("status.yaml")
    }
}
