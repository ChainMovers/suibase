use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use base64ct::{Base64UrlUnpadded, Encoding as Base64Encoding};

use fastcrypto::encoding::{Encoding as FastCryptoEncoding, Hex};

use anyhow::{anyhow, Result};

/// Returns the number of chars in Base64 unpadded to represent `byte_length` bytes.
pub const fn base64_len(byte_length: usize) -> usize {
    // Formula: ceiling(byte_length * 8 / 6) or ceiling(byte_length * 4 / 3)
    (byte_length * 4 + 2) / 3
}

/// Returns the number of chars in Hex to represent `byte_length` bytes. Excludes "0x" prefix.
pub const fn hex_len(byte_length: usize) -> usize {
    // Formula: ceiling(byte_length * 8 / 6) or ceiling(byte_length * 4 / 3)
    byte_length * 2
}

// Protocol constants that must be same on both client/server.
pub const ACOINS_PROTOCOL_V1: u8 = 1;
pub const ACOINS_PROTOCOL_VERSION_LATEST: u8 = ACOINS_PROTOCOL_V1;

pub const ACOINS_CHALLENGE_BYTES_LENGTH: usize = 12;
pub const ACOINS_CHALLENGE_STRING_LENGTH: usize = base64_len(ACOINS_CHALLENGE_BYTES_LENGTH);

pub const ACOINS_PK_BYTES_LENGTH: usize = 32;
pub const ACOINS_PK_STRING_LENGTH: usize = base64_len(ACOINS_PK_BYTES_LENGTH);

pub const ACOINS_PK_SHORT_BYTES_LENGTH: usize = 6;
pub const ACOINS_PK_SHORT_STRING_LENGTH: usize = base64_len(ACOINS_PK_SHORT_BYTES_LENGTH);

pub const ACOINS_SIGNATURE_BYTES_LENGTH: usize = 64;
pub const ACOINS_SIGNATURE_STRING_LENGTH: usize = base64_len(ACOINS_SIGNATURE_BYTES_LENGTH); // = 86

pub const ACOINS_SUI_ADDRESS_BYTES_LENGTH: usize = 32;
pub const ACOINS_SUI_ADDRESS_STRING_LENGTH: usize = base64_len(ACOINS_SUI_ADDRESS_BYTES_LENGTH);
pub const ACOINS_SUI_ADDRESS_HEX_LENGTH: usize = hex_len(ACOINS_SUI_ADDRESS_BYTES_LENGTH);

// Constants related to user specific challenges.
pub const ACOINS_STORAGE_NB_FILES: u8 = 25;
pub const ACOINS_STORAGE_FILE_SIZE: usize = 20 * 1024 * 1024;

// Constants related to versioned installed file challenges.
pub const ACOINS_CHALLENGE_FILE_SCRIPT_GLOBALS: u8 = 250;
pub const ACOINS_CHALLENGE_FILE_TSITE_BIN: u8 = 251;
pub const ACOINS_CHALLENGE_FILE_TWALRUS_BIN: u8 = 252;
pub const ACOINS_CHALLENGE_FILE_TSUI_BIN: u8 = 253;
pub const ACOINS_CHALLENGE_FILE_SUIBASE_BIN: u8 = 254;

// The response for the challenge.
//
// When FILE ID is > ACOINS_STORAGE_NB_DILES, this is encoded as:
//  - 3 bytes for <major>.<minor>.<patch> version installed.
//  - 5 bytes digest of the file requested.
//
// When a FILE ID is <= ACOINS_STORAGE_NB_DILES, this is encoded as:
//  - 8 bytes read at the challenge location.
//
pub const ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH: usize = 8;
pub const ACOINS_CHALLENGE_RESPONSE_STRING_LENGTH: usize =
    base64_len(ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH);

// Production address and ports
pub const ACOINS_SERVER_PUBLIC_URL: &str = "https://autocoins.suibase.io";
pub const ACOINS_SERVER_PUBLIC_API_PORT: u16 = 44400;
pub const ACOINS_SERVER_PUBLIC_DOWNLOAD_PORT: u16 = 44401;
pub const ACOINS_SERVER_PUBLIC_DISTRIBUTOR_PORT: u16 = 44402;
pub const ACOINS_SERVER_PUBLIC_ADMIN_PORT: u16 = 44403;

// Temporary local ports used for integration testing.
pub const ACOINS_SERVER_TEST_URL: &str = "http://127.0.0.1";
pub const ACOINS_SERVER_TEST_API_PORT: u16 = 44410;
pub const ACOINS_SERVER_TEST_DOWNLOAD_PORT: u16 = 44411;
pub const ACOINS_SERVER_TEST_DISTRIBUTOR_PORT: u16 = 44412;
pub const ACOINS_SERVER_TEST_ADMIN_PORT: u16 = 44413;

// Default local server ports (not public), for pre-production testing.
pub const ACOINS_SERVER_STAGE_URL: &str = "http://127.0.0.1";
pub const ACOINS_SERVER_STAGE_API_PORT: u16 = 44420;
pub const ACOINS_SERVER_STAGE_DOWNLOAD_PORT: u16 = 44421;
pub const ACOINS_SERVER_STAGE_DISTRIBUTOR_PORT: u16 = 44422;
pub const ACOINS_SERVER_STAGE_ADMIN_PORT: u16 = 44423;

// Utility to convert a Sui address ("0x"+64 hex chars) to a base64 unpadded string.
pub fn sui_address_hex_to_base64(sui_address_hex: &str) -> Result<String> {
    if sui_address_hex.len() != (ACOINS_SUI_ADDRESS_HEX_LENGTH + 2)
        && sui_address_hex.len() != ACOINS_SUI_ADDRESS_HEX_LENGTH
    {
        let err_msg = format!(
            "Invalid SUI address length {}. Expected {} or {}",
            sui_address_hex.len(),
            ACOINS_SUI_ADDRESS_HEX_LENGTH,
            ACOINS_SUI_ADDRESS_HEX_LENGTH + 2
        );
        return Err(anyhow::anyhow!(err_msg));
    }
    let bytes = Hex::decode(sui_address_hex)?;

    let mut buffer = [0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    buffer.copy_from_slice(&bytes);

    Ok(Base64UrlUnpadded::encode_string(&buffer))
}

// Utility to convert a base64 unpadded string (often used as JSON-RPC param)
// to a [u8; ACOINS_PK_BYTES_LENGTH]
pub fn pk_bytes_from_base64(pk_base64: &str) -> Result<[u8; ACOINS_PK_BYTES_LENGTH]> {
    if pk_base64.len() != ACOINS_PK_STRING_LENGTH {
        return Err(anyhow!("Invalid public key length pk={}", pk_base64));
    }

    // Decode the base64 string into a byte array.
    let mut buffer = [0u8; ACOINS_PK_BYTES_LENGTH];
    match Base64UrlUnpadded::decode(pk_base64, &mut buffer) {
        Ok(_) => Ok(buffer),
        Err(e) => {
            return Err(anyhow!("failed to decode base64 pk={}: {}", pk_base64, e));
        }
    }
}
// Utility to convert a base64 unpadded string (often used as JSON-RPC param)
// to a [u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH]
pub fn sui_address_bytes_from_base64(
    sui_address_base64: &str,
) -> Result<[u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH]> {
    if sui_address_base64.len() != ACOINS_SUI_ADDRESS_STRING_LENGTH {
        return Err(anyhow!(
            "Invalid sui_address base64 length {} != expected {} for [{}]",
            sui_address_base64.len(),
            ACOINS_SUI_ADDRESS_STRING_LENGTH,
            sui_address_base64
        ));
    }

    // Decode the base64 string into a byte array.
    let mut buffer = [0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    match Base64UrlUnpadded::decode(sui_address_base64, &mut buffer) {
        Ok(_) => Ok(buffer),
        Err(e) => {
            return Err(anyhow!("failed to decode base64 pk={}: {}", sui_address_base64, e));
        }
    }
}

pub struct ACoinsChallenge {
    day_offset: u32,
    file_offset: u32,
    file_number: u8, // one-based
    length: u8,
    flag0: u8,
    flag1: u8,
}

impl ACoinsChallenge {
    pub fn new(
        day_offset: u32,
        file_offset: u32,
        file_number: u8,
        length: u8,
        flag0: u8,
        flag1: u8,
    ) -> Self {
        Self {
            day_offset,
            file_offset,
            file_number,
            length,
            flag0,
            flag1,
        }
    }

    pub fn to_base64(&self) -> String {
        let buffer = self.to_bytes();
        Base64UrlUnpadded::encode_string(&buffer)
    }

    // Convert a base64 string into a ACoinsChallenge struct
    // Use base64ct.
    pub fn from_base64(base64: &str) -> Self {
        let mut buffer = [0u8; ACOINS_CHALLENGE_BYTES_LENGTH];
        if Base64UrlUnpadded::decode(base64, &mut buffer).is_err() {
            return {
                Self {
                    day_offset: 0,
                    file_offset: 0,
                    file_number: 0,
                    length: 0,
                    flag0: 0,
                    flag1: 0,
                }
            };
        }
        Self::from_bytes(&buffer)
    }

    pub fn from_bytes(bytes: &[u8; ACOINS_CHALLENGE_BYTES_LENGTH]) -> Self {
        let day_offset = ((bytes[0] as u32) << 24)
            | ((bytes[1] as u32) << 16)
            | ((bytes[2] as u32) << 8)
            | bytes[3] as u32;
        let file_offset = ((bytes[4] as u32) << 24)
            | ((bytes[5] as u32) << 16)
            | ((bytes[6] as u32) << 8)
            | bytes[7] as u32;
        let file_number = bytes[8];
        let length = bytes[9];
        let flag0 = bytes[10];
        let flag1 = bytes[11];

        Self {
            day_offset,
            file_offset,
            file_number,
            length,
            flag0,
            flag1,
        }
    }

    pub fn to_bytes(&self) -> [u8; ACOINS_CHALLENGE_BYTES_LENGTH] {
        let mut buffer = [0u8; ACOINS_CHALLENGE_BYTES_LENGTH];
        // Big-endian serialization.
        buffer[0] = (self.day_offset() >> 24) as u8;
        buffer[1] = (self.day_offset() >> 16) as u8;
        buffer[2] = (self.day_offset() >> 8) as u8;
        buffer[3] = self.day_offset() as u8;
        buffer[4] = (self.file_offset() >> 24) as u8;
        buffer[5] = (self.file_offset() >> 16) as u8;
        buffer[6] = (self.file_offset() >> 8) as u8;
        buffer[7] = self.file_offset() as u8;
        buffer[8] = self.file_number();
        buffer[9] = self.length();
        buffer[10] = self.flag0();
        buffer[11] = self.flag1();
        buffer
    }

    pub fn day_offset(&self) -> u32 {
        self.day_offset
    }

    pub fn file_offset(&self) -> u32 {
        self.file_offset
    }

    pub fn file_number(&self) -> u8 {
        self.file_number
    }

    pub fn length(&self) -> u8 {
        self.length
    }

    pub fn flag0(&self) -> u8 {
        self.flag0
    }

    pub fn flag1(&self) -> u8 {
        self.flag1
    }

    // Utility function to just extract the file_number from ACoinsChallenge encoded as bytes.
    pub fn file_number_from_bytes(bytes: &[u8; ACOINS_CHALLENGE_BYTES_LENGTH]) -> u8 {
        bytes[8]
    }
}

// Protocol responses from Proof-of-installation(POI) server.
#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    // Base 64 encoding of the location and length to retreive
    // from storage.
    //
    // The challenge for a given user changes every 24h.
    //
    // On any failure, the client protocol waits 24h before retrying
    // to avoid being banned by the server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
}

impl LoginResponse {
    pub fn new() -> Self {
        Self { challenge: None }
    }
}

impl Default for LoginResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub pass: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_fn: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_id: Option<String>,

    // Status info on POI actions.
    // Provided to client only when pass succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tsui_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tsui_deposit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twal_deposit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsui_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dsui_deposit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dwal_deposit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msui_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msui_deposit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mwal_deposit: Option<u64>,
}

impl VerifyResponse {
    pub fn new() -> Self {
        Self {
            pass: false,
            download_fn: None,
            download_id: None,
            tsui_address: None,
            tsui_deposit: None,
            twal_deposit: None,
            dsui_address: None,
            dsui_deposit: None,
            dwal_deposit: None,
            msui_address: None,
            msui_deposit: None,
            mwal_deposit: None,
        }
    }
}

impl Default for VerifyResponse {
    fn default() -> Self {
        Self::new()
    }
}
