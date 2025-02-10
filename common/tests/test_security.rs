#[cfg(test)]
mod tests {
    use common::security::{sign_value, generate_hmac_key};

    #[test]
    fn test_sign_value() {
        let message = "test message".to_string();
        let secret = "test secret".to_string();
        let signature = sign_value(message.clone(), secret.clone());
        assert_ne!(signature, "");
    }

    #[test]
    fn test_generate_hmac_key() {
        let key_len: u16 = 32;
        let key = generate_hmac_key(key_len);
        assert_eq!(key.len(), key_len as usize * 2); // each byte is represented by 2 hex chars
    }

    #[test]
    fn test_validate_signature() {
        let message = "test message".to_string();
        let secret = "test secret".to_string();
        let signature = sign_value(message.clone(), secret.clone());
        
        macro_rules! validate_signature {
            ($actual_signature:expr, $pre_signed:expr, $secret:expr) => {{
                let expected = common::security::sign_value($pre_signed.to_string(), $secret.to_string());
                expected == $actual_signature
            }};
        }

        assert!(validate_signature!(signature, message, secret));
    }
}
