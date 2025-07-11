// the version code is an internal code in integer format
// NOTE:
// - Server and client version codes could be different, but they must be compatible.
// - Increment the version for any change in the module
pub const CLIENT_VERSION_CODE: usize = 1;
pub const MIN_SERVER_VERSION_CODE: usize = 1;

pub fn get_client_version_code() -> usize {
    std::env::var("TEST_CLIENT_VERSION_CODE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(CLIENT_VERSION_CODE)
}

pub fn get_min_server_version_code() -> usize {
    std::env::var("TEST_MIN_SERVER_VERSION_CODE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(MIN_SERVER_VERSION_CODE)
}
