
use hmac::{Hmac, Mac};
use rand::Rng;
use sha2::Sha256;

// create a signature of any value using provided secret
// this is used by client service to create signature for server handshake
pub fn sign_value(message: String, secret: String) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("Error creating HMAC with the provided key.");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    
    hex::encode(result.into_bytes())
}

// generate strong key for hmac
// this will be used for server service to generate the key
pub fn generate_hmac_key(len: u16) -> String {
    // generate random bytes
    let mut rng = rand::thread_rng();
    let key: Vec<u8> = (0..len).map(|_| rng.gen()).collect();
    
    // converet to hex string
    let key_hex = key.iter().map(|b| format!("{:02x}", b)).collect::<String>();

    key_hex
}

// validate signature with provided pre-signed value and secret 
#[macro_export]
macro_rules! validate_signature {
    ($actual_signature:expr, $pre_signed:expr, $secret:expr) => {{
        let expected = $crate::security::sign_value($pre_signed, $secret);
        expected == $actual_signature
    }};
}
