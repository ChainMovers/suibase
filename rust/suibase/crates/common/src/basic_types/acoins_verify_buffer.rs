// Client/Server common code related to the "verify" step of the POI protocol.
use super::{
    UserKeypair, ACOINS_CHALLENGE_BYTES_LENGTH, ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH,
    ACOINS_CHALLENGE_RESPONSE_STRING_LENGTH, ACOINS_CHALLENGE_STRING_LENGTH,
    ACOINS_PK_BYTES_LENGTH, ACOINS_PK_STRING_LENGTH, ACOINS_SIGNATURE_BYTES_LENGTH,
    ACOINS_SIGNATURE_STRING_LENGTH, ACOINS_SUI_ADDRESS_BYTES_LENGTH,
    ACOINS_SUI_ADDRESS_STRING_LENGTH,
};
use arrayref::{array_mut_ref, array_ref};
use base64ct::{Base64UrlUnpadded, Encoding};
use fastcrypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    traits::{ToFromBytes, VerifyingKey},
};

/// Structure that help sign/verify fields used in "verify" messages
///
/// Optimized for speed by using a single buffer for all fields.
pub struct ACoinsVerifyBuffer {
    buffer: [u8; ACOINS_CHALLENGE_BYTES_LENGTH
        + ACOINS_PK_BYTES_LENGTH
        + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 3)
        + ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH
        + 4], // 8 bits req number + 24 bits flags.
    challenge_str: String,
}

impl ACoinsVerifyBuffer {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 3)
                + ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH
                + 4],
            challenge_str: String::new(),
        }
    }

    /// Access the parts of the buffer as mutable
    pub fn challenge_mut(&mut self) -> &mut [u8; ACOINS_CHALLENGE_BYTES_LENGTH] {
        array_mut_ref!(self.buffer, 0, ACOINS_CHALLENGE_BYTES_LENGTH)
    }

    pub fn pk_mut(&mut self) -> &mut [u8; ACOINS_PK_BYTES_LENGTH] {
        array_mut_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH,
            ACOINS_PK_BYTES_LENGTH
        )
    }

    pub fn devnet_address_mut(&mut self) -> &mut [u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_mut_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH + ACOINS_PK_BYTES_LENGTH,
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn testnet_address_mut(&mut self) -> &mut [u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_mut_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + ACOINS_SUI_ADDRESS_BYTES_LENGTH,
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn mainnet_address_mut(&mut self) -> &mut [u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_mut_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 2),
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn challenge_response_mut(&mut self) -> &mut [u8; ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH] {
        array_mut_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 3),
            ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH
        )
    }

    pub fn flags_mut(&mut self) -> &mut [u8; 4] {
        let offset = ACOINS_CHALLENGE_BYTES_LENGTH
            + ACOINS_PK_BYTES_LENGTH
            + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 3)
            + ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH;
        array_mut_ref!(self.buffer, offset, 4)
    }

    /// Access the parts as immutable
    pub fn challenge(&self) -> &[u8; ACOINS_CHALLENGE_BYTES_LENGTH] {
        array_ref!(self.buffer, 0, ACOINS_CHALLENGE_BYTES_LENGTH)
    }

    pub fn pk(&self) -> &[u8; ACOINS_PK_BYTES_LENGTH] {
        array_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH,
            ACOINS_PK_BYTES_LENGTH
        )
    }

    pub fn devnet_address(&self) -> &[u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH + ACOINS_PK_BYTES_LENGTH,
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn testnet_address(&self) -> &[u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + ACOINS_SUI_ADDRESS_BYTES_LENGTH,
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn mainnet_address(&self) -> &[u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 2),
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn challenge_response(&self) -> &[u8; ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH] {
        array_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 3),
            ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH
        )
    }

    pub fn flags(&self) -> &[u8; 4] {
        let offset = ACOINS_CHALLENGE_BYTES_LENGTH
            + ACOINS_PK_BYTES_LENGTH
            + (ACOINS_SUI_ADDRESS_BYTES_LENGTH * 3)
            + ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH;
        array_ref!(self.buffer, offset, 4)
    }

    // Conveniently process flags as u32.
    pub fn get_flags_as_u32(&self) -> u32 {
        u32::from_le_bytes(*self.flags())
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags_mut().copy_from_slice(&flags.to_le_bytes());
    }

    // Access to the req file_number field (8 MSB of flags)
    pub fn req_file(&self) -> u8 {
        let flags = self.get_flags_as_u32();
        ((flags >> 24) & 0xFF) as u8
    }

    pub fn set_req_file(&mut self, file_number: u8) {
        let current = self.get_flags_as_u32();
        // Clear the top 8 bits and set them to the file number
        let new_flags = (current & 0x00FFFFFF) | ((file_number as u32) << 24);
        self.set_flags(new_flags);
    }

    // Flag operations on the lower 24 bits
    pub fn set_flag(&mut self, flag_bit: u8) -> Result<(), &'static str> {
        if flag_bit >= 24 {
            return Err("Flag bit must be in range 0-23");
        }
        let flags = self.get_flags_as_u32();
        let new_flags = flags | (1u32 << flag_bit);
        self.set_flags(new_flags);
        Ok(())
    }

    pub fn clear_flag(&mut self, flag_bit: u8) -> Result<(), &'static str> {
        if flag_bit >= 24 {
            return Err("Flag bit must be in range 0-23");
        }
        let flags = self.get_flags_as_u32();
        let new_flags = flags & !(1u32 << flag_bit);
        self.set_flags(new_flags);
        Ok(())
    }

    pub fn has_flag(&self, flag_bit: u8) -> Result<bool, &'static str> {
        if flag_bit >= 24 {
            return Err("Flag bit must be in range 0-23");
        }
        let flags = self.get_flags_as_u32();
        Ok((flags & (1u32 << flag_bit)) != 0)
    }

    /// Initialize the challenge part from a base64 string
    pub fn set_challenge_from_base64(&mut self, challenge_str: String) -> Result<(), &'static str> {
        if challenge_str.len() != ACOINS_CHALLENGE_STRING_LENGTH {
            return Err("Invalid challenge length");
        }

        Base64UrlUnpadded::decode(&challenge_str, self.challenge_mut())
            .map_err(|_| "Error decoding challenge")?;

        // Keep ownership of the string
        self.challenge_str = challenge_str;
        Ok(())
    }

    pub fn challenge_str(&self) -> &str {
        &self.challenge_str
    }

    /// Initialize the public key part from a base64 string
    pub fn set_pk_from_base64(&mut self, pk: &str) -> Result<(), &'static str> {
        if pk.len() != ACOINS_PK_STRING_LENGTH {
            return Err("Invalid public key length");
        }

        Base64UrlUnpadded::decode(pk, self.pk_mut()).map_err(|_| "Error decoding public key")?;
        Ok(())
    }

    /// Initialize the deposit address part from a base64 string
    pub fn set_devnet_address_from_base64(&mut self, address: &str) -> Result<(), &'static str> {
        if address.len() != ACOINS_SUI_ADDRESS_STRING_LENGTH {
            return Err("Invalid devnet address length");
        }

        Base64UrlUnpadded::decode(address, self.devnet_address_mut())
            .map_err(|_| "Error decoding devnet address")?;
        Ok(())
    }

    /// Initialize the testnet address part from a base64 string
    pub fn set_testnet_address_from_base64(&mut self, address: &str) -> Result<(), &'static str> {
        if address.len() != ACOINS_SUI_ADDRESS_STRING_LENGTH {
            return Err("Invalid testnet address length");
        }

        Base64UrlUnpadded::decode(address, self.testnet_address_mut())
            .map_err(|_| "Error decoding testnet address")?;
        Ok(())
    }

    /// Initialize the mainnet address part from a base64 string
    pub fn set_mainnet_address_from_base64(&mut self, address: &str) -> Result<(), &'static str> {
        if address.len() != ACOINS_SUI_ADDRESS_STRING_LENGTH {
            return Err("Invalid mainnet address length");
        }

        Base64UrlUnpadded::decode(address, self.mainnet_address_mut())
            .map_err(|_| "Error decoding mainnet address")?;
        Ok(())
    }

    /// Initialize the data/response part from a base64 string if provided
    pub fn set_challenge_response_from_base64(
        &mut self,
        data: Option<&str>,
    ) -> Result<(), &'static str> {
        if let Some(data) = data {
            if data.len() != ACOINS_CHALLENGE_RESPONSE_STRING_LENGTH {
                return Err("Invalid data length");
            }

            Base64UrlUnpadded::decode(data, self.challenge_response_mut())
                .map_err(|_| "Error decoding data")?;
        }
        // If None, the data buffer remains zeroed
        Ok(())
    }

    /// Verify a signature against this buffer
    pub fn verify_signature(&self, signature: &str) -> bool {
        if signature.len() != ACOINS_SIGNATURE_STRING_LENGTH {
            return false;
        }

        let mut sig_bytes = [0u8; ACOINS_SIGNATURE_BYTES_LENGTH];
        if Base64UrlUnpadded::decode(signature, &mut sig_bytes).is_err() {
            return false;
        }

        self.inner_verify_signature(&self.buffer, self.pk(), &sig_bytes)
    }

    fn inner_verify_signature(
        &self,
        data: &[u8],
        pk_bytes: &[u8; ACOINS_PK_BYTES_LENGTH],
        sig_bytes: &[u8; ACOINS_SIGNATURE_BYTES_LENGTH],
    ) -> bool {
        match Ed25519PublicKey::from_bytes(pk_bytes) {
            Ok(pk) => match Ed25519Signature::from_bytes(sig_bytes) {
                Ok(sig) => pk.verify(data, &sig).is_ok(),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    /// Sign the buffer, and return as a base64 string
    pub fn sign(&self, user_keypair: &UserKeypair) -> String {
        user_keypair.sign(&self.buffer)
    }

    /// Get a reference to the entire buffer
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }
}
