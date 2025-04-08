use base64ct::{Base64UrlUnpadded, Encoding};

#[cfg(test)]
mod tests {
    use common::basic_types::{
        ACoinsChallenge, ACOINS_CHALLENGE_BYTES_LENGTH, ACOINS_CHALLENGE_STRING_LENGTH,
        ACOINS_STORAGE_NB_FILES,
    };

    use super::*;

    #[test]
    fn test_acoins_challenge_roundtrip() {
        // Test with standard values
        let challenge = ACoinsChallenge::new(
            0x12345678, // day_offset
            0x87654321, // file_offset
            5,          // file_number (1-based)
            64,         // length
            0x42,       // flag0
            0xFF,       // flag1
        );

        // Test to_base64 and from_base64
        let base64 = challenge.to_base64();
        assert_eq!(base64.len(), ACOINS_CHALLENGE_STRING_LENGTH);

        let decoded = ACoinsChallenge::from_base64(&base64);

        // Verify all fields are preserved
        assert_eq!(decoded.day_offset(), 0x12345678);
        assert_eq!(decoded.file_offset(), 0x87654321);
        assert_eq!(decoded.file_number(), 5);
        assert_eq!(decoded.length(), 64);
        assert_eq!(decoded.flag0(), 0x42);
        assert_eq!(decoded.flag1(), 0xFF);
    }

    #[test]
    fn test_acoins_challenge_bytes_roundtrip() {
        // Test with different values
        let challenge = ACoinsChallenge::new(
            0x01020304, // day_offset
            0xA0B0C0D0, // file_offset
            25,         // file_number (max value)
            128,        // length
            0x01,       // flag0
            0x02,       // flag1
        );

        // Convert to bytes
        let mut buffer = [0u8; ACOINS_CHALLENGE_BYTES_LENGTH];
        buffer[0] = (challenge.day_offset() >> 24) as u8;
        buffer[1] = (challenge.day_offset() >> 16) as u8;
        buffer[2] = (challenge.day_offset() >> 8) as u8;
        buffer[3] = challenge.day_offset() as u8;
        buffer[4] = (challenge.file_offset() >> 24) as u8;
        buffer[5] = (challenge.file_offset() >> 16) as u8;
        buffer[6] = (challenge.file_offset() >> 8) as u8;
        buffer[7] = challenge.file_offset() as u8;
        buffer[8] = challenge.file_number();
        buffer[9] = challenge.length();
        buffer[10] = challenge.flag0();
        buffer[11] = challenge.flag1();

        // Reconstruct from bytes
        let decoded = ACoinsChallenge::from_bytes(&buffer);

        // Verify all fields are preserved
        assert_eq!(decoded.day_offset(), 0x01020304);
        assert_eq!(decoded.file_offset(), 0xA0B0C0D0);
        assert_eq!(decoded.file_number(), 25);
        assert_eq!(decoded.length(), 128);
        assert_eq!(decoded.flag0(), 0x01);
        assert_eq!(decoded.flag1(), 0x02);
    }

    #[test]
    fn test_file_number_extraction() {
        // Test different file numbers
        for file_num in 1..=ACOINS_STORAGE_NB_FILES {
            // Create a challenge with the specific file number
            let challenge = ACoinsChallenge::new(
                0x12345678, // day_offset
                0x87654321, // file_offset
                file_num,   // file_number - testing different values
                64,         // length
                0x42,       // flag0
                0xFF,       // flag1
            );

            // Test the optimization of getting the file_number directly from the challenge bytes.
            let buffer = challenge.to_bytes();
            assert_eq!(ACoinsChallenge::file_number_from_bytes(&buffer), file_num);
        }
    }

    #[test]
    fn test_boundary_values() {
        // Test with boundary values
        let challenge = ACoinsChallenge::new(
            0,          // min day_offset
            0xFFFFFFFF, // max file_offset
            1,          // min file_number
            0,          // min length
            0,          // min flag0
            0,          // min flag1
        );

        let base64 = challenge.to_base64();
        let decoded = ACoinsChallenge::from_base64(&base64);

        assert_eq!(decoded.day_offset(), 0);
        assert_eq!(decoded.file_offset(), 0xFFFFFFFF);
        assert_eq!(decoded.file_number(), 1);
        assert_eq!(decoded.length(), 0);
        assert_eq!(decoded.flag0(), 0);
        assert_eq!(decoded.flag1(), 0);

        // Test max values
        let challenge2 = ACoinsChallenge::new(
            0xFFFFFFFF,              // max day_offset
            0,                       // min file_offset
            ACOINS_STORAGE_NB_FILES, // max file_number
            255,                     // max length (u8)
            255,                     // max flag0 (u8)
            255,                     // max flag1 (u8)
        );

        let base64_2 = challenge2.to_base64();
        let decoded2 = ACoinsChallenge::from_base64(&base64_2);

        assert_eq!(decoded2.day_offset(), 0xFFFFFFFF);
        assert_eq!(decoded2.file_offset(), 0);
        assert_eq!(decoded2.file_number(), ACOINS_STORAGE_NB_FILES);
        assert_eq!(decoded2.length(), 255);
        assert_eq!(decoded2.flag0(), 255);
        assert_eq!(decoded2.flag1(), 255);
    }

    #[test]
    fn test_invalid_base64() {
        // Test handling of invalid base64 string
        let invalid_base64 = "ThisIsNotValidBase64!@#";
        let challenge = ACoinsChallenge::from_base64(invalid_base64);

        // Should return a zeroed challenge
        assert_eq!(challenge.day_offset(), 0);
        assert_eq!(challenge.file_offset(), 0);
        assert_eq!(challenge.file_number(), 0);
        assert_eq!(challenge.length(), 0);
        assert_eq!(challenge.flag0(), 0);
        assert_eq!(challenge.flag1(), 0);
    }

    #[test]
    fn test_base64_formatting() {
        // Create a challenge with known values
        let challenge = ACoinsChallenge::new(0x12345678, 0x87654321, 5, 64, 0x42, 0xFF);

        // Generate base64
        let base64 = challenge.to_base64();

        // Create the expected bytes
        let expected_bytes = [
            0x12, 0x34, 0x56, 0x78, // day_offset
            0x87, 0x65, 0x43, 0x21, // file_offset
            0x05, // file_number
            0x40, // length
            0x42, // flag0
            0xFF, // flag1
        ];

        // Generate expected base64
        let expected_base64 = Base64UrlUnpadded::encode_string(&expected_bytes);

        // Verify
        assert_eq!(base64, expected_base64);
    }
}
