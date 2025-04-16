use std::path::Path;

use base64ct::{Base64UrlUnpadded, Encoding as Base64Encoding};
use secrecy::{ExposeSecret, SecretBox};
use zeroize::{Zeroize, ZeroizeOnDrop};

use fastcrypto::{
    ed25519::Ed25519KeyPair,
    encoding::{Base58, Encoding as FastCryptoEncoding},
    traits::{KeyPair, Signer, ToFromBytes},
};
use rand::rngs::StdRng;
use rand::SeedableRng;

use anyhow::{anyhow, Result};

use crate::log_safe;
// This wrapper implement Zeroize for Ed25519KeyPair
#[derive(ZeroizeOnDrop)]
pub struct ZeroizableKeypair {
    keypair_bytes: Vec<u8>,
    pk_bytes: Vec<u8>,
    pk_string_base64: String,
}

impl ZeroizableKeypair {
    pub fn new(kp: Ed25519KeyPair) -> Self {
        let pk_bytes_ref = kp.public().as_bytes();
        Self {
            keypair_bytes: kp.as_bytes().to_vec(), // Keep secret in a zeroizable way.
            pk_bytes: pk_bytes_ref.to_vec(),
            pk_string_base64: Base64UrlUnpadded::encode_string(pk_bytes_ref),
        }
    }

    fn as_ed25519_keypair(&self) -> Ed25519KeyPair {
        Ed25519KeyPair::from_bytes(&self.keypair_bytes).unwrap()
    }

    pub fn pk_to_string(&self) -> String {
        self.pk_string_base64.clone()
    }

    pub fn pk_as_str(&self) -> &str {
        &self.pk_string_base64
    }

    pub fn pk_as_bytes(&self) -> &[u8] {
        &self.pk_bytes
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
        // Note: Unecessary to zeroize public key fields (pk_*).
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

    pub fn from_string(keypair_str: &str) -> Result<Self> {
        // Decode the base58 string to bytes, return error if invalid
        let keypair_bytes = Base58::decode(keypair_str)
            .map_err(|_| anyhow::anyhow!("Invalid Base58 encoding in keypair string"))?;

        // Convert bytes to Ed25519KeyPair, return error if invalid
        let kp = Ed25519KeyPair::from_bytes(&keypair_bytes)
            .map_err(|_| anyhow::anyhow!("Invalid Ed25519 keypair bytes"))?;

        let zeroizable_kp = ZeroizableKeypair::new(kp);
        Ok(Self {
            kp: SecretBox::new(Box::new(zeroizable_kp)),
        })
    }

    /// Load keypair from file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Read file contents asynchronously
        let mut content = tokio::fs::read_to_string(path).await?;

        // Trim any whitespace or newlines and convert to keypair
        let keypair_str = content.trim();

        // Convert to keypair - propagate any errors
        let new_user_keypair = Self::from_string(keypair_str)?;
        content.zeroize();
        Ok(new_user_keypair)
    }

    /// Save keypair to file (with secure permissions).
    pub async fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if path.as_ref().exists() {
            return Err(anyhow::anyhow!("Refusing to overwrite existing keypair"));
        }

        // Create parent directories if needed
        if let Some(parent) = path.as_ref().parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        {
            let mut keypair_str = self.to_string();

            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;

                // Use std::fs with secure permissions, then use tokio for async operations
                let temp_path = path.as_ref().with_extension(".tmp");
                {
                    let mut options = std::fs::OpenOptions::new();
                    let mut file = options
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .mode(0o600) // Secure from creation
                        .open(&temp_path)?;

                    use std::io::Write;
                    file.write_all(keypair_str.as_bytes())?;
                }

                tokio::fs::rename(temp_path, path).await?;
            }

            #[cfg(not(unix))]
            {
                // On non-Unix platforms, just write the file
                tokio::fs::write(path, keypair_str).await?;
            }

            keypair_str.zeroize();
        }

        Ok(())
    }

    pub fn to_string(&self) -> String {
        let binding = self.kp.expose_secret().as_ed25519_keypair();
        let keypair_bytes = binding.as_bytes();
        Base58::encode(keypair_bytes)
    }

    pub fn pk_to_string(&self) -> String {
        self.kp.expose_secret().pk_to_string()
    }

    pub fn pk_as_bytes(&self) -> &[u8] {
        self.kp.expose_secret().pk_as_bytes()
    }

    pub fn sign(&self, data: &[u8]) -> String {
        let binding = self.kp.expose_secret().as_ed25519_keypair();
        Base64UrlUnpadded::encode_string(binding.sign(data).as_ref())
    }
}

impl Default for UserKeypair {
    fn default() -> Self {
        Self::new()
    }
}

// LocalUserKeyPair is a factory of a UserKeypair that succeed only if
// a "user.keypair" file is loaded/created at the specified path.
pub struct LocalUserKeyPair {
    kp: UserKeypair,
}

impl LocalUserKeyPair {
    pub async fn from_file<P: AsRef<Path>>(user_keypair_file: P) -> Result<Self> {
        let mut user_keypair: Option<UserKeypair> = None;

        // Atempt, up to 3 times, to get a user.keypair file loaded+validated.
        let mut attempts = 0;
        let mut verified_invalid = false;
        while attempts < 3 && user_keypair.is_none() {
            attempts += 1;
            // Delete a user.keypair file it it was verified invalid (in a previous iteration).
            // This is for recovery on FS corruption or user messing up with the file.
            if verified_invalid {
                if user_keypair_file.as_ref().exists() {
                    if let Err(error) = tokio::fs::remove_file(&user_keypair_file).await {
                        let err_msg = format!(
                            "failed to delete file {}: {}",
                            user_keypair_file.as_ref().display(),
                            error
                        );
                        log_safe!(err_msg);
                        continue;
                    }
                }
                verified_invalid = false;
            }

            if !user_keypair_file.as_ref().exists() {
                let new_user_keypair = UserKeypair::new();

                if let Err(error) = new_user_keypair.to_file(&user_keypair_file).await {
                    let err_msg = format!(
                        "failed to write keypair to file {}: {}",
                        user_keypair_file.as_ref().display(),
                        error
                    );
                    log_safe!(err_msg);
                    continue;
                }
            }

            // Always load directly from fs to validate (even if just created above).
            let read_back = UserKeypair::from_file(&user_keypair_file).await;
            if let Err(error) = read_back {
                let err_msg = format!(
                    "failed to read keypair from file {}: {}",
                    user_keypair_file.as_ref().display(),
                    error
                );
                log_safe!(err_msg);
                verified_invalid = true;
                continue;
            }
            user_keypair = Some(read_back.unwrap());
        }

        if user_keypair.is_none() {
            let err_msg = format!(
                "failed to load user.keypair from '{}'",
                user_keypair_file.as_ref().display()
            );
            return Err(anyhow!(err_msg));
        }

        Ok(Self {
            kp: user_keypair.unwrap(),
        })
    }

    pub fn get_kp(&self) -> &UserKeypair {
        &self.kp
    }
}
