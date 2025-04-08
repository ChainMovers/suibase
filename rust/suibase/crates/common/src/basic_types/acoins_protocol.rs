use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use base64ct::{Base64UrlUnpadded, Encoding as Base64Encoding};

use fastcrypto::encoding::{Encoding as FastCryptoEncoding, Hex};

use anyhow::Result;

/// Returns the number of chars in Base64 unpadded to represent `byte_length` bytes.
pub const fn base64_len(byte_length: usize) -> usize {
    // Formula: ceiling(byte_length * 8 / 6) or ceiling(byte_length * 4 / 3)
    (byte_length * 4 + 2) / 3
}

// Protocol constants that must be same on both client/server.
pub const ACOINS_PROTOCOL_V1: u8 = 1;
pub const ACOINS_PROTOCOL_VERSION_LATEST: u8 = ACOINS_PROTOCOL_V1;

pub const ACOINS_STORAGE_NB_FILES: u8 = 25;
pub const ACOINS_STORAGE_FILE_SIZE: usize = 20 * 1024 * 1024;

pub const ACOINS_CHALLENGE_BYTES_LENGTH: usize = 12;
pub const ACOINS_CHALLENGE_STRING_LENGTH: usize = base64_len(ACOINS_CHALLENGE_BYTES_LENGTH);

pub const ACOINS_PK_BYTES_LENGTH: usize = 32;
pub const ACOINS_PK_STRING_LENGTH: usize = base64_len(ACOINS_PK_BYTES_LENGTH);

pub const ACOINS_PK_SHORT_BYTES_LENGTH: usize = 6;
pub const ACOINS_PK_SHORT_STRING_LENGTH: usize = base64_len(ACOINS_PK_SHORT_BYTES_LENGTH);

pub const ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH: usize = 8;
pub const ACOINS_CHALLENGE_RESPONSE_STRING_LENGTH: usize =
    base64_len(ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH);

pub const ACOINS_SIGNATURE_BYTES_LENGTH: usize = 64;
pub const ACOINS_SIGNATURE_STRING_LENGTH: usize = base64_len(ACOINS_SIGNATURE_BYTES_LENGTH); // = 86

pub const ACOINS_SUI_ADDRESS_BYTES_LENGTH: usize = 32;
pub const ACOINS_SUI_ADDRESS_STRING_LENGTH: usize = base64_len(ACOINS_SUI_ADDRESS_BYTES_LENGTH);

pub const ACOINS_SERVER_PORT_API: u16 = 44400;
pub const ACOINS_SERVER_PORT_DOWNLOAD: u16 = 44401;

// Utility to convert a Sui address ("0x"+64 hex chars) to a base64 unpadded string.
pub fn sui_address_to_base64(sui_address: &str) -> Result<String> {
    if sui_address.len() != 66 && sui_address.len() != 64 {
        return Err(anyhow::anyhow!("Invalid SUI address length"));
    }
    let bytes = Hex::decode(sui_address)?;

    let mut buffer = [0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    buffer.copy_from_slice(&bytes);

    Ok(Base64UrlUnpadded::encode_string(&buffer))
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

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq, Default)]
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
            ..Default::default()
        }
    }
}
