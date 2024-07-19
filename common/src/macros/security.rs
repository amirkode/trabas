
use hmac::{Hmac, Mac};
use sha2::Sha256;

// create a signature of any value using provided secret
// this is used by client service to create signature for server handshake
pub fn sign_value_func(message: String, secret: String) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("Error creating HMAC with the provided key.");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    
    hex::encode(result.into_bytes())
}

// export the sing_message function as a macro
#[macro_export]
macro_rules! sign_value {
    ($value:expr, $secret:expr) => {{
        $crate::macros::security::sign_value_func($message, $secret)
    }};
}

// validate signature with provided pre-signed value and secret 
#[macro_export]
macro_rules! validate_signature {
    ($actual_signature:expr, $pre_signed:expr, $secret:expr) => {{
        let expected = $crate::macros::security::sign_value_func($pre_signed, $secret);
        expected == $actual_signature
    }};
}
