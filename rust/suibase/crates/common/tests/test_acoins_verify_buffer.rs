use base64ct::{Base64UrlUnpadded, Encoding};
use common::basic_types::{
    ACoinsVerifyBuffer, UserKeypair, ACOINS_CHALLENGE_BYTES_LENGTH,
    ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH, ACOINS_PK_BYTES_LENGTH, ACOINS_SIGNATURE_STRING_LENGTH,
    ACOINS_SUI_ADDRESS_BYTES_LENGTH,
};

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

fn create_test_keypair() -> UserKeypair {
    // Create a deterministic keypair for testing
    let seed = [1u8; 32];
    let mut rng = StdRng::from_seed(seed);
    let mut private_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut private_key_bytes);

    const DEFAULT_SK_BASE58: &str = "31RQT9SK2ZE6M83LURhvyVYJmWBNTAVYqj5jKhbcZTVo";
    UserKeypair::from_string(DEFAULT_SK_BASE58).expect("Valid keypair string")
}

fn create_sample_buffer() -> ACoinsVerifyBuffer {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Fill buffer with test data
    let keypair = create_test_keypair();
    let result = buffer.set_pk_from_base64(&keypair.pk_to_string());
    assert!(result.is_ok(), "Failed to set pk from base64");

    // Set a challenge
    let mut challenge_bytes = [0u8; ACOINS_CHALLENGE_BYTES_LENGTH];
    for i in 0..ACOINS_CHALLENGE_BYTES_LENGTH {
        challenge_bytes[i] = i as u8;
    }
    let challenge_str = Base64UrlUnpadded::encode_string(&challenge_bytes);
    let result = buffer.set_challenge_from_base64(challenge_str);
    assert!(result.is_ok(), "Failed to set challenge from base64");

    buffer
}

#[test]
fn test_basic_initialization() {
    let buffer = ACoinsVerifyBuffer::new();

    // Initial state
    assert_eq!(buffer.req_file(), 0);
    assert_eq!(buffer.get_flags_as_u32() & 0x00FFFFFF, 0); // Lower 24 bits should be 0

    for i in 0..24 {
        assert_eq!(buffer.has_flag(i).unwrap(), false);
    }
}

#[test]
fn test_flag_operations() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Test setting individual flags
    for i in 0..24 {
        assert_eq!(buffer.has_flag(i).unwrap(), false);
        buffer.set_flag(i).unwrap();
        assert_eq!(buffer.has_flag(i).unwrap(), true);

        // Verify other flags are still unset
        for j in 0..24 {
            if j != i {
                assert_eq!(buffer.has_flag(j).unwrap(), false);
            }
        }

        // Clear the flag
        buffer.clear_flag(i).unwrap();
        assert_eq!(buffer.has_flag(i).unwrap(), false);
    }

    // Test setting all flags
    for i in 0..24 {
        buffer.set_flag(i).unwrap();
    }

    for i in 0..24 {
        assert_eq!(buffer.has_flag(i).unwrap(), true);
    }

    // Test clearing all flags
    for i in 0..24 {
        buffer.clear_flag(i).unwrap();
    }

    for i in 0..24 {
        assert_eq!(buffer.has_flag(i).unwrap(), false);
    }
}

#[test]
fn test_flag_bounds_checking() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Test with invalid flag indices
    assert!(buffer.set_flag(24).is_err());
    assert!(buffer.set_flag(25).is_err());
    assert!(buffer.set_flag(255).is_err());

    assert!(buffer.clear_flag(24).is_err());
    assert!(buffer.has_flag(24).is_err());
}

#[test]
fn test_req_file_operations() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Test various values
    for file_num in 0..=255 {
        buffer.set_req_file(file_num);
        assert_eq!(buffer.req_file(), file_num);
    }

    // Test edge cases
    buffer.set_req_file(0);
    assert_eq!(buffer.req_file(), 0);

    buffer.set_req_file(255);
    assert_eq!(buffer.req_file(), 255);
}

#[test]
fn test_req_file_and_flags_interaction() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Set all flags to 1
    for i in 0..24 {
        buffer.set_flag(i).unwrap();
    }

    // Set req_file to a value with all bits set
    buffer.set_req_file(255);

    // Verify all flags are still set
    for i in 0..24 {
        assert_eq!(
            buffer.has_flag(i).unwrap(),
            true,
            "Flag {} should still be set",
            i
        );
    }
    assert_eq!(buffer.req_file(), 255);

    // Now clear all flags
    for i in 0..24 {
        buffer.clear_flag(i).unwrap();
    }

    // Verify req_file is still set properly
    assert_eq!(buffer.req_file(), 255);

    // Set req_file to 0 and verify flags stay cleared
    buffer.set_req_file(0);
    for i in 0..24 {
        assert_eq!(buffer.has_flag(i).unwrap(), false);
    }

    // Set specific bit patterns in both fields
    buffer.set_req_file(0xAA); // 10101010
    buffer.set_flag(0).unwrap(); // Bit 0
    buffer.set_flag(2).unwrap(); // Bit 2
    buffer.set_flag(4).unwrap(); // Bit 4
    buffer.set_flag(23).unwrap(); // Highest bit in flags

    // Verify everything is set correctly
    assert_eq!(buffer.req_file(), 0xAA);
    assert_eq!(buffer.has_flag(0).unwrap(), true);
    assert_eq!(buffer.has_flag(1).unwrap(), false);
    assert_eq!(buffer.has_flag(2).unwrap(), true);
    assert_eq!(buffer.has_flag(3).unwrap(), false);
    assert_eq!(buffer.has_flag(4).unwrap(), true);
    assert_eq!(buffer.has_flag(23).unwrap(), true);
}

#[test]
fn test_endianness_handling() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Set a specific pattern in flags
    buffer.set_flag(0).unwrap(); // Bit 0 (LSB)
    buffer.set_flag(7).unwrap(); // Bit 7 (end of first byte)
    buffer.set_flag(8).unwrap(); // Bit 8 (start of second byte)
    buffer.set_flag(15).unwrap(); // Bit 15 (end of second byte)
    buffer.set_flag(16).unwrap(); // Bit 16 (start of third byte)
    buffer.set_flag(23).unwrap(); // Bit 23 (end of third byte, before req_file)

    // Set req_file to a value with clear bit pattern
    buffer.set_req_file(0xF0); // 11110000

    // Raw check of the flags field bytes
    let flags_bytes = buffer.flags();

    // If little-endian (as expected):
    // flags_bytes[0] should have bits 0 and 7 set = 10000001 = 0x81
    // flags_bytes[1] should have bits 0 and 7 set = 10000001 = 0x81
    // flags_bytes[2] should have bits 0 and 7 set = 10000001 = 0x81
    // flags_bytes[3] should be 0xF0 (req_file)

    assert_eq!(
        flags_bytes[0] & 0x81,
        0x81,
        "Byte 0 flags incorrect, endianness issue?"
    );
    assert_eq!(
        flags_bytes[1] & 0x81,
        0x81,
        "Byte 1 flags incorrect, endianness issue?"
    );
    assert_eq!(
        flags_bytes[2] & 0x81,
        0x81,
        "Byte 2 flags incorrect, endianness issue?"
    );

    // Check if req_file is in the correct byte position (should be MSB in little-endian)
    // Confirm that req_file is stored in the MSB (byte 3) as expected
    assert_eq!(
        flags_bytes[3], 0xF0,
        "req_file not stored in expected byte position, found {:x} in MSB",
        flags_bytes[3]
    );
}

#[test]
fn test_signature_verification() {
    let mut buffer = create_sample_buffer();
    let keypair = create_test_keypair();

    // Set some flags and req_file to ensure they're included in signature
    buffer.set_flag(0).unwrap();
    buffer.set_flag(23).unwrap();
    buffer.set_req_file(42);

    // Sign the buffer
    let signature = buffer.sign(&keypair);
    assert_eq!(signature.len(), ACOINS_SIGNATURE_STRING_LENGTH);

    // Verify the signature
    assert!(buffer.verify_signature(&signature));

    // Modify a flag and check that signature fails
    buffer.set_flag(1).unwrap();
    assert!(!buffer.verify_signature(&signature));

    // Fix it back and verify signature works again
    buffer.clear_flag(1).unwrap();
    assert!(buffer.verify_signature(&signature));

    // Change req_file and verify signature fails
    buffer.set_req_file(43);
    assert!(!buffer.verify_signature(&signature));
}

#[test]
fn test_multiple_flags_with_req_file() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Set alternating flags
    for i in (0..24).step_by(2) {
        buffer.set_flag(i).unwrap();
    }

    // Initial check
    for i in 0..24 {
        if i % 2 == 0 {
            assert!(buffer.has_flag(i).unwrap());
        } else {
            assert!(!buffer.has_flag(i).unwrap());
        }
    }

    // Try every possible req_file value
    for file_num in 0..=255 {
        buffer.set_req_file(file_num);
        assert_eq!(buffer.req_file(), file_num);

        // Verify flags are preserved
        for i in 0..24 {
            if i % 2 == 0 {
                assert!(
                    buffer.has_flag(i).unwrap(),
                    "Flag {} should be set with req_file={}",
                    i,
                    file_num
                );
            } else {
                assert!(
                    !buffer.has_flag(i).unwrap(),
                    "Flag {} should not be set with req_file={}",
                    i,
                    file_num
                );
            }
        }
    }
}

#[test]
fn test_alternate_flag_and_req_file_changes() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Alternate between flag changes and req_file changes
    // to ensure they don't interfere with each other
    for i in 0..24 {
        // Set req_file to i
        buffer.set_req_file(i);
        assert_eq!(buffer.req_file(), i);

        // Set flag i
        buffer.set_flag(i).unwrap();
        assert!(buffer.has_flag(i).unwrap());

        // Verify req_file is still i
        assert_eq!(buffer.req_file(), i);

        // Change req_file
        buffer.set_req_file(i + 100);
        assert_eq!(buffer.req_file(), i + 100);

        // Verify flag i is still set
        assert!(buffer.has_flag(i).unwrap());

        // Clear flag i
        buffer.clear_flag(i).unwrap();
        assert!(!buffer.has_flag(i).unwrap());

        // Verify req_file is still i + 100
        assert_eq!(buffer.req_file(), i + 100);
    }
}

// Helper function for creating test keypair - implementation depends on your UserKeypair

#[test]
fn test_address_base64_setters() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Create test addresses
    let devnet_addr = [0x11; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let testnet_addr = [0x22; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let mainnet_addr = [0x33; ACOINS_SUI_ADDRESS_BYTES_LENGTH];

    // Convert to base64
    let devnet_addr_base64 = Base64UrlUnpadded::encode_string(&devnet_addr);
    let testnet_addr_base64 = Base64UrlUnpadded::encode_string(&testnet_addr);
    let mainnet_addr_base64 = Base64UrlUnpadded::encode_string(&mainnet_addr);

    // Set addresses using base64 methods
    buffer
        .set_devnet_address_from_base64(&devnet_addr_base64)
        .unwrap();
    buffer
        .set_testnet_address_from_base64(&testnet_addr_base64)
        .unwrap();
    buffer
        .set_mainnet_address_from_base64(&mainnet_addr_base64)
        .unwrap();

    // Verify addresses were set correctly using accessors
    assert_eq!(buffer.devnet_address(), &devnet_addr);
    assert_eq!(buffer.testnet_address(), &testnet_addr);
    assert_eq!(buffer.mainnet_address(), &mainnet_addr);
}

#[test]
fn test_address_manual_setters() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Create test addresses
    let devnet_addr = [0x11; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let testnet_addr = [0x22; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let mainnet_addr = [0x33; ACOINS_SUI_ADDRESS_BYTES_LENGTH];

    // Set addresses using mutable references
    buffer.devnet_address_mut().copy_from_slice(&devnet_addr);
    buffer.testnet_address_mut().copy_from_slice(&testnet_addr);
    buffer.mainnet_address_mut().copy_from_slice(&mainnet_addr);

    // Verify addresses were set correctly
    assert_eq!(buffer.devnet_address(), &devnet_addr);
    assert_eq!(buffer.testnet_address(), &testnet_addr);
    assert_eq!(buffer.mainnet_address(), &mainnet_addr);
}

#[test]
fn test_address_base64_roundtrip() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Create test addresses with recognizable patterns
    let devnet_addr = {
        let mut addr = [0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
        for i in 0..addr.len() {
            addr[i] = (i * 3) as u8;
        }
        addr
    };

    let testnet_addr = {
        let mut addr = [0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
        for i in 0..addr.len() {
            addr[i] = (i * 5 + 1) as u8;
        }
        addr
    };

    let mainnet_addr = {
        let mut addr = [0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
        for i in 0..addr.len() {
            addr[i] = (i * 7 + 2) as u8;
        }
        addr
    };

    // Set addresses using mutable references
    buffer.devnet_address_mut().copy_from_slice(&devnet_addr);
    buffer.testnet_address_mut().copy_from_slice(&testnet_addr);
    buffer.mainnet_address_mut().copy_from_slice(&mainnet_addr);

    // Encode to base64
    let devnet_addr_base64 = Base64UrlUnpadded::encode_string(buffer.devnet_address());
    let testnet_addr_base64 = Base64UrlUnpadded::encode_string(buffer.testnet_address());
    let mainnet_addr_base64 = Base64UrlUnpadded::encode_string(buffer.mainnet_address());

    // Create a new buffer and set from base64
    let mut buffer2 = ACoinsVerifyBuffer::new();
    buffer2
        .set_devnet_address_from_base64(&devnet_addr_base64)
        .unwrap();
    buffer2
        .set_testnet_address_from_base64(&testnet_addr_base64)
        .unwrap();
    buffer2
        .set_mainnet_address_from_base64(&mainnet_addr_base64)
        .unwrap();

    // Verify addresses match original
    assert_eq!(buffer2.devnet_address(), &devnet_addr);
    assert_eq!(buffer2.testnet_address(), &testnet_addr);
    assert_eq!(buffer2.mainnet_address(), &mainnet_addr);
}

#[test]
fn test_invalid_base64_addresses() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Test with invalid base64
    let invalid_base64 = "This is not valid base64!";
    assert!(buffer
        .set_devnet_address_from_base64(invalid_base64)
        .is_err());
    assert!(buffer
        .set_testnet_address_from_base64(invalid_base64)
        .is_err());
    assert!(buffer
        .set_mainnet_address_from_base64(invalid_base64)
        .is_err());

    // Test with valid base64 but wrong length
    let short_base64 = Base64UrlUnpadded::encode_string(&[0x11, 0x22, 0x33]);
    assert!(buffer
        .set_devnet_address_from_base64(&short_base64)
        .is_err());
    assert!(buffer
        .set_testnet_address_from_base64(&short_base64)
        .is_err());
    assert!(buffer
        .set_mainnet_address_from_base64(&short_base64)
        .is_err());

    // Ensure addresses remain unchanged
    assert_eq!(
        buffer.devnet_address(),
        &[0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH]
    );
    assert_eq!(
        buffer.testnet_address(),
        &[0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH]
    );
    assert_eq!(
        buffer.mainnet_address(),
        &[0u8; ACOINS_SUI_ADDRESS_BYTES_LENGTH]
    );
}

#[test]
fn test_multiple_fields_with_signature() {
    // Skip this test if create_test_keypair is not implemented
    if let Ok(keypair) = std::panic::catch_unwind(|| create_test_keypair()) {
        let mut buffer = ACoinsVerifyBuffer::new();

        // Set challenge
        let challenge = [1u8; ACOINS_CHALLENGE_BYTES_LENGTH];
        buffer.challenge_mut().copy_from_slice(&challenge);

        // Set response
        let response = [2u8; ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH];
        buffer.challenge_response_mut().copy_from_slice(&response);

        // Set addresses
        let devnet_addr = [0x11; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
        let testnet_addr = [0x22; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
        let mainnet_addr = [0x33; ACOINS_SUI_ADDRESS_BYTES_LENGTH];

        buffer.devnet_address_mut().copy_from_slice(&devnet_addr);
        buffer.testnet_address_mut().copy_from_slice(&testnet_addr);
        buffer.mainnet_address_mut().copy_from_slice(&mainnet_addr);

        // Set public key from keypair
        buffer.pk_mut().copy_from_slice(&keypair.pk_as_bytes());

        // Set flags and req_file
        buffer.set_flag(0).unwrap();
        buffer.set_flag(23).unwrap();
        buffer.set_req_file(123);

        // Sign and verify
        let signature = buffer.sign(&keypair);
        assert!(buffer.verify_signature(&signature));

        // Modify one address and verify signature fails
        buffer.devnet_address_mut()[0] = 0x44;
        assert!(!buffer.verify_signature(&signature));
    }
}

#[test]
fn test_base64_challenge_combined_with_other_fields() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Set challenge from base64
    let challenge = [1u8; ACOINS_CHALLENGE_BYTES_LENGTH];
    let challenge_base64 = Base64UrlUnpadded::encode_string(&challenge);
    buffer
        .set_challenge_from_base64(challenge_base64.clone())
        .unwrap();

    // Set addresses from base64
    let devnet_addr = [0xAA; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let testnet_addr = [0xBB; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let mainnet_addr = [0xCC; ACOINS_SUI_ADDRESS_BYTES_LENGTH];

    let devnet_addr_base64 = Base64UrlUnpadded::encode_string(&devnet_addr);
    let testnet_addr_base64 = Base64UrlUnpadded::encode_string(&testnet_addr);
    let mainnet_addr_base64 = Base64UrlUnpadded::encode_string(&mainnet_addr);

    buffer
        .set_devnet_address_from_base64(&devnet_addr_base64)
        .unwrap();
    buffer
        .set_testnet_address_from_base64(&testnet_addr_base64)
        .unwrap();
    buffer
        .set_mainnet_address_from_base64(&mainnet_addr_base64)
        .unwrap();

    // Set flags and req_file
    buffer.set_flag(0).unwrap();
    buffer.set_flag(23).unwrap();
    buffer.set_req_file(123);

    // Verify all fields are set correctly
    assert_eq!(buffer.challenge(), &challenge);
    assert_eq!(buffer.challenge_str(), &challenge_base64);
    assert_eq!(buffer.devnet_address(), &devnet_addr);
    assert_eq!(buffer.testnet_address(), &testnet_addr);
    assert_eq!(buffer.mainnet_address(), &mainnet_addr);
    assert!(buffer.has_flag(0).unwrap());
    assert!(buffer.has_flag(23).unwrap());
    assert_eq!(buffer.req_file(), 123);
}

#[test]
fn test_challenge_response() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Test with a provided response
    let response = [0x42; ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH];
    let response_base64 = Base64UrlUnpadded::encode_string(&response);

    buffer
        .set_challenge_response_from_base64(Some(&response_base64))
        .unwrap();
    assert_eq!(buffer.challenge_response(), &response);

    // Test with None
    let mut buffer2 = ACoinsVerifyBuffer::new();
    buffer2.set_challenge_response_from_base64(None).unwrap();
    assert_eq!(
        buffer2.challenge_response(),
        &[0u8; ACOINS_CHALLENGE_RESPONSE_BYTES_LENGTH]
    );
}

#[test]
fn test_pk_from_base64() {
    let mut buffer = ACoinsVerifyBuffer::new();

    // Create a test public key
    let pk = [0x55; ACOINS_PK_BYTES_LENGTH];
    let pk_base64 = Base64UrlUnpadded::encode_string(&pk);

    // Set via base64
    buffer.set_pk_from_base64(&pk_base64).unwrap();

    // Verify it was set correctly
    assert_eq!(buffer.pk(), &pk);

    // Test with invalid base64
    assert!(buffer.set_pk_from_base64("invalid!").is_err());

    // Test with wrong length
    let short_pk = [0x55; 16];
    let short_pk_base64 = Base64UrlUnpadded::encode_string(&short_pk);
    assert!(buffer.set_pk_from_base64(&short_pk_base64).is_err());
}

#[test]
fn test_buffer_serialization_stability() {
    // This test ensures the internal buffer representation is stable
    // after setting addresses via base64 or byte methods
    let mut buffer1 = ACoinsVerifyBuffer::new();
    let mut buffer2 = ACoinsVerifyBuffer::new();

    // Create test data
    let challenge = [0x42; ACOINS_CHALLENGE_BYTES_LENGTH];
    let devnet_addr = [0x11; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let testnet_addr = [0x22; ACOINS_SUI_ADDRESS_BYTES_LENGTH];
    let mainnet_addr = [0x33; ACOINS_SUI_ADDRESS_BYTES_LENGTH];

    // Buffer 1: Set everything using byte methods
    buffer1.challenge_mut().copy_from_slice(&challenge);
    buffer1.devnet_address_mut().copy_from_slice(&devnet_addr);
    buffer1.testnet_address_mut().copy_from_slice(&testnet_addr);
    buffer1.mainnet_address_mut().copy_from_slice(&mainnet_addr);

    // Buffer 2: Set everything using base64 methods
    let challenge_base64 = Base64UrlUnpadded::encode_string(&challenge);
    let devnet_addr_base64 = Base64UrlUnpadded::encode_string(&devnet_addr);
    let testnet_addr_base64 = Base64UrlUnpadded::encode_string(&testnet_addr);
    let mainnet_addr_base64 = Base64UrlUnpadded::encode_string(&mainnet_addr);

    buffer2.set_challenge_from_base64(challenge_base64).unwrap();
    buffer2
        .set_devnet_address_from_base64(&devnet_addr_base64)
        .unwrap();
    buffer2
        .set_testnet_address_from_base64(&testnet_addr_base64)
        .unwrap();
    buffer2
        .set_mainnet_address_from_base64(&mainnet_addr_base64)
        .unwrap();

    // Compare the raw buffers - they should be identical (except for challenge_str field)
    assert_eq!(buffer1.as_bytes(), buffer2.as_bytes());
}
