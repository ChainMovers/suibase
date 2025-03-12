use secrecy::{ExposeSecret, SecretBox};
use zeroize::{Zeroize, ZeroizeOnDrop};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use base64ct::{Base64UrlUnpadded, Encoding as Base64Encoding};

use fastcrypto::{
    ed25519::{Ed25519KeyPair, Ed25519PublicKey, Ed25519Signature},
    encoding::{Base58, Encoding as FastCryptoEncoding},
    traits::{KeyPair, Signer, ToFromBytes, VerifyingKey},
};

use rand::rngs::StdRng;
use rand::SeedableRng;

/// Returns the number of chars in Base64 unpadded to represent `byte_length` bytes.
pub const fn base64_len(byte_length: usize) -> usize {
    // Formula: ceiling(byte_length * 8 / 6) or ceiling(byte_length * 4 / 3)
    (byte_length * 4 + 2) / 3
}

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

pub struct ACoinsChallenge {
    day_offset: u32,
    file_offset: u32,
    file_number: u8,
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
        let mut buffer = [0u8; ACOINS_CHALLENGE_BYTES_LENGTH];
        // Big-endian serialization.
        buffer[0] = (self.day_offset >> 24) as u8;
        buffer[1] = (self.day_offset >> 16) as u8;
        buffer[2] = (self.day_offset >> 8) as u8;
        buffer[3] = self.day_offset as u8;
        buffer[4] = (self.file_offset >> 24) as u8;
        buffer[5] = (self.file_offset >> 16) as u8;
        buffer[6] = (self.file_offset >> 8) as u8;
        buffer[7] = self.file_offset as u8;
        buffer[8] = self.file_number;
        buffer[9] = self.length;
        buffer[10] = self.flag0;
        buffer[11] = self.flag1;
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

// This wrapper implement Zeroize for Ed25519KeyPair
#[derive(ZeroizeOnDrop)]
pub struct ZeroizableKeypair {
    keypair_bytes: Vec<u8>,
}

impl ZeroizableKeypair {
    pub fn new(kp: Ed25519KeyPair) -> Self {
        Self {
            keypair_bytes: kp.as_bytes().to_vec(),
        }
    }

    fn as_ed25519_keypair(&self) -> Ed25519KeyPair {
        Ed25519KeyPair::from_bytes(&self.keypair_bytes).unwrap()
    }

    pub fn pk_to_string(&self) -> String {
        // TODO Security: keypair still exposed on the stack.
        let pk_bytes = self.as_ed25519_keypair().public().as_bytes().to_vec();
        Base64UrlUnpadded::encode_string(&pk_bytes)
    }

    // The signature is base64 encoded.
    pub fn sign(&self, data: &[u8]) -> String {
        let sign_obj = self.as_ed25519_keypair().sign(data);
        Base64UrlUnpadded::encode_string(sign_obj.as_ref())
    }
}

impl Zeroize for ZeroizableKeypair {
    fn zeroize(&mut self) {
        self.keypair_bytes.zeroize();
    }
}

#[derive(ZeroizeOnDrop)]
pub struct UserKeypair {
    kp: SecretBox<ZeroizableKeypair>,
}

impl UserKeypair {
    pub fn new() -> Self {
        let kp = Ed25519KeyPair::generate(&mut StdRng::from_entropy());
        let zeroizable_kp = ZeroizableKeypair::new(kp);
        Self {
            kp: SecretBox::new(Box::new(zeroizable_kp)),
        }
    }

    // TODO Security: Should implement from/to file to avoid further exposing private key.
    pub fn from_string(keypair_str: &str) -> Self {
        let keypair_bytes = Base58::decode(keypair_str).unwrap();
        let kp = Ed25519KeyPair::from_bytes(&keypair_bytes).unwrap();
        let zeroizable_kp = ZeroizableKeypair::new(kp);
        Self {
            kp: SecretBox::new(Box::new(zeroizable_kp)),
        }
    }

    pub fn to_string(&self) -> String {
        let binding = self.kp.expose_secret().as_ed25519_keypair();
        let keypair_bytes = binding.as_bytes();
        Base58::encode(keypair_bytes)
    }

    pub fn pk_to_string(&self) -> String {
        self.kp.expose_secret().pk_to_string()
    }

    pub fn sign(&self, data: &[u8]) -> String {
        let binding = self.kp.expose_secret().as_ed25519_keypair();
        Base64UrlUnpadded::encode_string(binding.sign(data).as_ref())
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
    // The challenge for a given user changes every 12h.
    //
    // On any failure, must wait 12h before retrying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
}

impl LoginResponse {
    pub fn new() -> Self {
        Self { challenge: None }
    }
}

#[serde_as]
#[derive(Clone, Debug, JsonSchema, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifyResponse {
    pub pass: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_id: Option<String>,
}

impl VerifyResponse {
    pub fn new() -> Self {
        Self {
            pass: false,
            download_id: None,
        }
    }
}
