// NOTE:
// - Version is the build version of the binary or project.
// - Server and client versions could be different, but they must be compatible.

use common::version::get_root_version;

const MIN_SERVER_VERSION: &str = "0.2.0-beta.2";

pub fn get_client_version() -> String {
    std::env::var("TEST_CLIENT_VERSION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(get_root_version())
}

pub fn get_min_server_version() -> String {
    std::env::var("TEST_MIN_SERVER_VERSION")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(MIN_SERVER_VERSION.to_string())
}
