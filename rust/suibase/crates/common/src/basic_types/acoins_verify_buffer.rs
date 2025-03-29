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
        + ACOINS_SUI_ADDRESS_BYTES_LENGTH
        + ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH],
}

impl ACoinsVerifyBuffer {
    pub fn new() -> Self {
        Self {
            buffer: [0u8; ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + ACOINS_SUI_ADDRESS_BYTES_LENGTH
                + ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH],
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

    pub fn deposit_address_mut(&mut self) -> &mut [u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_mut_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH + ACOINS_PK_BYTES_LENGTH,
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn challenge_response_mut(&mut self) -> &mut [u8; ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH] {
        array_mut_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + ACOINS_SUI_ADDRESS_BYTES_LENGTH,
            ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH
        )
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

    pub fn deposit_address(&self) -> &[u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH] {
        array_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH + ACOINS_PK_BYTES_LENGTH,
            ACOINS_SUI_ADDRESS_BYTES_LENGTH
        )
    }

    pub fn challenge_response(&self) -> &[u8; ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH] {
        array_ref!(
            self.buffer,
            ACOINS_CHALLENGE_BYTES_LENGTH
                + ACOINS_PK_BYTES_LENGTH
                + ACOINS_SUI_ADDRESS_BYTES_LENGTH,
            ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH
        )
    }

    /// Initialize the challenge part from a base64 string
    pub fn set_challenge_from_base64(&mut self, challenge: &str) -> Result<(), &'static str> {
        if challenge.len() != ACOINS_CHALLENGE_STRING_LENGTH {
            return Err("Invalid challenge length");
        }

        Base64UrlUnpadded::decode(challenge, self.challenge_mut())
            .map_err(|_| "Error decoding challenge")?;
        Ok(())
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
    pub fn set_deposit_address_from_base64(
        &mut self,
        deposit_address: &str,
    ) -> Result<(), &'static str> {
        if deposit_address.len() != ACOINS_SUI_ADDRESS_STRING_LENGTH {
            return Err("Invalid deposit address length");
        }

        Base64UrlUnpadded::decode(deposit_address, self.deposit_address_mut())
            .map_err(|_| "Error decoding deposit address")?;
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
