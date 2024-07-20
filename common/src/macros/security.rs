// validate signature with provided pre-signed value and secret 
#[macro_export]
macro_rules! validate_signature {
    ($actual_signature:expr, $pre_signed:expr, $secret:expr) => {{
        let expected = $crate::security::sign_value($pre_signed, $secret);
        expected == $actual_signature
    }};
}