#[cfg(test)]
mod tests {
    use std::time::UNIX_EPOCH;
    use common::data::dto::{
        cache_config::CacheConfig,
        cache::Cache,
        public_request::PublicRequest,
        public_response::PublicResponse,
        tunnel_ack::TunnelAck,
        tunnel_client::TunnelClient,
    };

    #[test]
    fn test_cache_config_serialization() {
        let cache_config = CacheConfig::new(
            "client123".to_string(),
            "GET".to_string(),
            "/api/test".to_string(),
            3600
        );

        let serialized = serde_json::to_string(&cache_config).expect("Failed to serialize CacheConfig");
        assert!(serialized.contains("client123"));
        assert!(serialized.contains("GET"));
        assert!(serialized.contains("/api/test"));
        assert!(serialized.contains("3600"));

        let deserialized: CacheConfig = serde_json::from_str(&serialized).expect("Failed to deserialize CacheConfig");
        assert_eq!(deserialized.client_id, cache_config.client_id);
        assert_eq!(deserialized.method, cache_config.method);
        assert_eq!(deserialized.path, cache_config.path);
        assert_eq!(deserialized.exp_duration, cache_config.exp_duration);
    }

    #[test]
    fn test_cache_serialization() {
        let test_time = UNIX_EPOCH + std::time::Duration::from_secs(1234567890);
        let test_data = b"test cache data".to_vec();
        let cache = Cache::new(test_time, test_data.clone());

        let serialized = serde_json::to_string(&cache).expect("Failed to serialize Cache");
        let deserialized: Cache = serde_json::from_str(&serialized).expect("Failed to deserialize Cache");
        assert_eq!(deserialized.data, test_data);
        let duration_diff = deserialized.expired_at.duration_since(cache.expired_at)
            .unwrap_or_else(|_| cache.expired_at.duration_since(deserialized.expired_at).unwrap());
        assert!(duration_diff.as_millis() < 1000);
    }

    #[test]
    fn test_public_request_serialization() {
        let request_id = "req_12345".to_string();
        let request_data = b"HTTP request body data".to_vec();
        let public_request = PublicRequest {
            id: request_id.clone(),
            data: request_data.clone(),
        };

        let serialized = serde_json::to_string(&public_request).expect("Failed to serialize PublicRequest");
        assert!(serialized.contains("req_12345"));

        let deserialized: PublicRequest = serde_json::from_str(&serialized).expect("Failed to deserialize PublicRequest");
        assert_eq!(deserialized.id, request_id);
        assert_eq!(deserialized.data, request_data);
    }

    #[test]
    fn test_public_response_serialization() {
        let response = PublicResponse::new(
            "req_12345".to_string(),
            "tunnel_67890".to_string(),
            b"HTTP response body".to_vec()
        );

        let serialized = serde_json::to_string(&response).expect("Failed to serialize PublicResponse");
        assert!(serialized.contains("req_12345"));
        assert!(serialized.contains("tunnel_67890"));

        let deserialized: PublicResponse = serde_json::from_str(&serialized).expect("Failed to deserialize PublicResponse");
        assert_eq!(deserialized.request_id, response.request_id);
        assert_eq!(deserialized.tunnel_id, response.tunnel_id);
        assert_eq!(deserialized.data, response.data);
    }

    #[test]
    fn test_public_response_empty_tunnel_id_serialization() {
        let response = PublicResponse {
            request_id: "req_12345".to_string(),
            tunnel_id: String::new(), // skiped, since it's empty
            data: b"test data".to_vec(),
        };

        let serialized = serde_json::to_string(&response).expect("Failed to serialize PublicResponse");
        assert!(!serialized.contains("tunnel_id"));
        assert!(serialized.contains("req_12345"));

        let deserialized: PublicResponse = serde_json::from_str(&serialized).expect("Failed to deserialize PublicResponse");
        assert_eq!(deserialized.request_id, "req_12345");
        assert_eq!(deserialized.tunnel_id, ""); // default to empty string
        assert_eq!(deserialized.data, b"test data".to_vec());
    }

    #[test]
    fn test_tunnel_ack_serialization() {
        let tunnel_ack = TunnelAck::success(
            "tunnel_123".to_string(),
            "client_mac_abc".to_string(),
            "server_secret_xyz".to_string(),
            vec!["/api/endpoint1".to_string(), "/api/endpoint2".to_string()]
        );

        let serialized = serde_json::to_string(&tunnel_ack).expect("Failed to serialize TunnelAck");
        assert!(serialized.contains("tunnel_123"));
        assert!(serialized.contains("true"));
        assert!(serialized.contains("/api/endpoint1"));
        assert!(serialized.contains("/api/endpoint2"));

        let deserialized: TunnelAck = serde_json::from_str(&serialized).expect("Failed to deserialize TunnelAck");
        assert_eq!(deserialized.id, tunnel_ack.id);
        assert_eq!(deserialized.success, tunnel_ack.success);
        assert_eq!(deserialized.message, tunnel_ack.message);
        assert_eq!(deserialized.public_endpoints, tunnel_ack.public_endpoints);
    }

    #[test]
    fn test_tunnel_ack_failure_serialization() {
        let tunnel_ack = TunnelAck::fails(
            "tunnel_456".to_string(),
            "Authentication failed".to_string(),
        );

        let serialized = serde_json::to_string(&tunnel_ack).expect("Failed to serialize TunnelAck");
        assert!(serialized.contains("tunnel_456"));
        assert!(serialized.contains("false"));
        assert!(serialized.contains("Authentication failed"));

        let deserialized: TunnelAck = serde_json::from_str(&serialized).expect("Failed to deserialize TunnelAck");
        assert_eq!(deserialized.id, tunnel_ack.id);
        assert_eq!(deserialized.success, false);
        assert_eq!(deserialized.message, tunnel_ack.message);
        assert!(deserialized.public_endpoints.is_empty());
    }

    #[test]
    fn test_tunnel_client_serialization() {
        let tunnel_client = TunnelClient::new(
            "client_abc123".to_string(),
            "signature_xyz789".to_string(),
            "1.0.0".to_string(),
            "0.5.0".to_string()
        );
        let serialized = serde_json::to_string(&tunnel_client).expect("Failed to serialize TunnelClient");
        assert!(serialized.contains("client_abc123"));
        assert!(serialized.contains("1.0.0"));
        assert!(serialized.contains("0.5.0"));

        let deserialized: TunnelClient = serde_json::from_str(&serialized).expect("Failed to deserialize TunnelClient");
        assert_eq!(deserialized.id, tunnel_client.id);
        assert_eq!(deserialized.signature, tunnel_client.signature);
        assert_eq!(deserialized.cl_version, tunnel_client.cl_version);
        assert_eq!(deserialized.min_sv_version, tunnel_client.min_sv_version);
        assert!(!deserialized.alias_id.is_empty());
        assert!(deserialized.conn_dc_at.is_none());
    }

    #[test]
    fn test_tunnel_client_version_validation() {
        let tunnel_client = TunnelClient::new(
            "client_test".to_string(),
            "test_signature".to_string(),
            "1.0.0".to_string(),
            "0.8.0".to_string()
        );

        // valid version combinations
        assert!(tunnel_client.validate_version("0.9.0".to_string(), "0.5.0".to_string())); // server >= min_sv, client >= min_cl
        assert!(tunnel_client.validate_version("0.8.0".to_string(), "1.0.0".to_string())); // exact minimum versions

        // invalid version combinations  
        assert!(!tunnel_client.validate_version("0.7.0".to_string(), "0.5.0".to_string())); // server < min_sv
        assert!(!tunnel_client.validate_version("0.9.0".to_string(), "1.5.0".to_string())); // client < min_cl
        assert!(!tunnel_client.validate_version("0.7.0".to_string(), "1.5.0".to_string())); // both invalid
    }

    #[test]
    fn test_tunnel_client_default_version_codes() {
        let json_without_versions = r#"{
            "id": "client_test",
            "alias_id": "alias123",
            "signature": "test_sig",
            "conn_est_at": {"secs_since_epoch": 1234567890, "nanos_since_epoch": 0},
            "conn_dc_at": null
        }"#;

        let deserialized: TunnelClient = serde_json::from_str(json_without_versions)
            .expect("Failed to deserialize TunnelClient with default versions");
        
        assert_eq!(deserialized.cl_version, "");
        assert_eq!(deserialized.min_sv_version, "");
        assert_eq!(deserialized.id, "client_test");
        assert_eq!(deserialized.alias_id, "alias123");
        assert_eq!(deserialized.signature, "test_sig");
    }

    #[test]
    fn test_cache_config_raw_json_deserialization() {
        let raw_json = r#"{
            "client_id": "client456",
            "method": "POST",
            "path": "/api/users",
            "exp_duration": 7200
        }"#;

        let deserialized: CacheConfig = serde_json::from_str(raw_json)
            .expect("Failed to deserialize CacheConfig from raw JSON");
        
        assert_eq!(deserialized.client_id, "client456");
        assert_eq!(deserialized.method, "POST");
        assert_eq!(deserialized.path, "/api/users");
        assert_eq!(deserialized.exp_duration, 7200);
    }

    #[test]
    fn test_cache_raw_json_deserialization() {
        let raw_json = r#"{
            "expired_at": {
                "secs_since_epoch": 1234567890,
                "nanos_since_epoch": 123456789
            },
            "data": [116, 101, 115, 116, 32, 100, 97, 116, 97]
        }"#;

        let deserialized: Cache = serde_json::from_str(raw_json)
            .expect("Failed to deserialize Cache from raw JSON");
        
        assert_eq!(deserialized.data, b"test data".to_vec());
        let expected_time = UNIX_EPOCH + std::time::Duration::new(1234567890, 123456789);
        assert_eq!(deserialized.expired_at, expected_time);
    }

    #[test]
    fn test_public_request_raw_json_deserialization() {
        let raw_json = r#"{
            "id": "req_789",
            "data": [72, 84, 84, 80, 32, 114, 101, 113, 117, 101, 115, 116]
        }"#;

        let deserialized: PublicRequest = serde_json::from_str(raw_json)
            .expect("Failed to deserialize PublicRequest from raw JSON");
        
        assert_eq!(deserialized.id, "req_789");
        assert_eq!(deserialized.data, b"HTTP request".to_vec());
    }

    #[test]
    fn test_public_response_raw_json_deserialization() {
        let raw_json = r#"{
            "request_id": "req_456",
            "tunnel_id": "tunnel_789",
            "data": [72, 84, 84, 80, 32, 114, 101, 115, 112, 111, 110, 115, 101]
        }"#;

        let deserialized: PublicResponse = serde_json::from_str(raw_json)
            .expect("Failed to deserialize PublicResponse from raw JSON");
        
        assert_eq!(deserialized.request_id, "req_456");
        assert_eq!(deserialized.tunnel_id, "tunnel_789");
        assert_eq!(deserialized.data, b"HTTP response".to_vec());
    }

    #[test]
    fn test_public_response_missing_tunnel_id_raw_json() {
        let raw_json = r#"{
            "request_id": "req_999",
            "data": [116, 101, 115, 116]
        }"#;

        let deserialized: PublicResponse = serde_json::from_str(raw_json)
            .expect("Failed to deserialize PublicResponse without tunnel_id");
        
        assert_eq!(deserialized.request_id, "req_999");
        assert_eq!(deserialized.tunnel_id, ""); // Should default to empty string
        assert_eq!(deserialized.data, b"test".to_vec());
    }

    #[test]
    fn test_tunnel_ack_raw_json_deserialization() {
        let raw_json = r#"{
            "id": "tunnel_success",
            "signature": "sig_123",
            "success": true,
            "message": "Tunnel established",
            "public_endpoints": ["/health", "/metrics", "/api/v1"]
        }"#;

        let deserialized: TunnelAck = serde_json::from_str(raw_json)
            .expect("Failed to deserialize TunnelAck from raw JSON");
        
        assert_eq!(deserialized.id, "tunnel_success");
        assert_eq!(deserialized.signature, "sig_123");
        assert_eq!(deserialized.success, true);
        assert_eq!(deserialized.message, "Tunnel established");
        assert_eq!(deserialized.public_endpoints, vec!["/health", "/metrics", "/api/v1"]);
    }

    #[test]
    fn test_tunnel_ack_failure_raw_json_deserialization() {
        let raw_json = r#"{
            "id": "tunnel_fail",
            "signature": "sig_fail",
            "success": false,
            "message": "Invalid credentials",
            "public_endpoints": []
        }"#;

        let deserialized: TunnelAck = serde_json::from_str(raw_json)
            .expect("Failed to deserialize TunnelAck failure from raw JSON");
        
        assert_eq!(deserialized.id, "tunnel_fail");
        assert_eq!(deserialized.signature, "sig_fail");
        assert_eq!(deserialized.success, false);
        assert_eq!(deserialized.message, "Invalid credentials");
        assert!(deserialized.public_endpoints.is_empty());
    }

    #[test]
    fn test_tunnel_client_raw_json_deserialization() {
        let raw_json = r#"{
            "id": "client_raw_test",
            "alias_id": "alias_raw_123",
            "signature": "sig_abc123",
            "conn_est_at": {
                "secs_since_epoch": 1640995200,
                "nanos_since_epoch": 0
            },
            "conn_dc_at": null,
            "cl_version": "1.5.0",
            "min_sv_version": "0.7.5"
        }"#;

        let deserialized: TunnelClient = serde_json::from_str(raw_json)
            .expect("Failed to deserialize TunnelClient from raw JSON");
        
        assert_eq!(deserialized.id, "client_raw_test");
        assert_eq!(deserialized.alias_id, "alias_raw_123");
        assert_eq!(deserialized.signature, "sig_abc123");
        assert_eq!(deserialized.cl_version, "1.5.0");
        assert_eq!(deserialized.min_sv_version, "0.7.5");
        assert!(deserialized.conn_dc_at.is_none());
    }

    #[test]
    fn test_tunnel_client_minimal_raw_json() {
        // Test with minimal required fields
        let raw_json = r#"{
            "id": "minimal_client",
            "alias_id": "minimal_alias",
            "signature": "minimal_sig",
            "conn_est_at": {
                "secs_since_epoch": 1640995200,
                "nanos_since_epoch": 0
            }
        }"#;

        let deserialized: TunnelClient = serde_json::from_str(raw_json)
            .expect("Failed to deserialize minimal TunnelClient from raw JSON");
        
        assert_eq!(deserialized.id, "minimal_client");
        assert_eq!(deserialized.alias_id, "minimal_alias");
        assert_eq!(deserialized.signature, "minimal_sig");
        assert_eq!(deserialized.cl_version, ""); // Should default to empty string
        assert_eq!(deserialized.min_sv_version, ""); // Should default to empty string
        assert!(deserialized.conn_dc_at.is_none());
    }

    #[test]
    fn test_tunnel_client_with_disconnect_time_raw_json() {
        let raw_json = r#"{
            "id": "disconnected_client",
            "alias_id": "disc_alias",
            "signature": "disc_sig",
            "conn_est_at": {
                "secs_since_epoch": 1640995200,
                "nanos_since_epoch": 0
            },
            "conn_dc_at": {
                "secs_since_epoch": 1640999800,
                "nanos_since_epoch": 0
            },
            "cl_version": "2.0.0",
            "min_sv_version": "1.0.0"
        }"#;

        let deserialized: TunnelClient = serde_json::from_str(raw_json)
            .expect("Failed to deserialize TunnelClient with disconnect time");
        
        assert_eq!(deserialized.id, "disconnected_client");
        assert!(deserialized.conn_dc_at.is_some());
        let disconnect_time = deserialized.conn_dc_at.unwrap();
        let expected_disconnect = UNIX_EPOCH + std::time::Duration::from_secs(1640999800);
        assert_eq!(disconnect_time, expected_disconnect);
    }

    #[test]
    fn test_tunnel_client_with_omitted_versions_raw_json() {
        let raw_json = r#"{
            "id": "disconnected_client",
            "alias_id": "disc_alias",
            "signature": "disc_sig",
            "conn_est_at": {
                "secs_since_epoch": 1640995200,
                "nanos_since_epoch": 0
            },
            "conn_dc_at": {
                "secs_since_epoch": 1640999800,
                "nanos_since_epoch": 0
            }
        }"#;

        let deserialized: TunnelClient = serde_json::from_str(raw_json)
            .expect("Failed to deserialize TunnelClient with disconnect time");
        
        assert_eq!(deserialized.id, "disconnected_client");
        assert!(deserialized.conn_dc_at.is_some());
        let disconnect_time = deserialized.conn_dc_at.unwrap();
        let expected_disconnect = UNIX_EPOCH + std::time::Duration::from_secs(1640999800);
        assert_eq!(disconnect_time, expected_disconnect);
        assert_eq!(deserialized.cl_version, "");
        assert_eq!(deserialized.min_sv_version, "");
    }
}
