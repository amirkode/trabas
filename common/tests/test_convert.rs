#[cfg(test)]
mod tests {
    use common::convert::*;
    use http::{Request, Response, Version};
    use std::collections::HashMap;

    #[test]
    fn test_parse_request_bytes() {
        let request_bytes = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let request = parse_request_bytes(request_bytes).unwrap();
        assert_eq!(request.method(), "GET");
        assert_eq!(request.uri().path(), "/");
        assert_eq!(request.version(), Version::HTTP_11);
        assert_eq!(request.headers().get("Host").unwrap(), "example.com");
    }

    #[test]
    fn test_request_to_bytes() {
        let request = Request::builder()
            .method("POST")
            .uri("/test")
            .version(Version::HTTP_11)
            .header("Content-Type", "application/json")
            .body(vec![123, 34, 107, 101, 121, 34, 58, 32, 34, 118, 97, 108, 117, 101, 34, 125]) // {"key": "value"}
            .unwrap();
        let bytes = request_to_bytes(&request);
        let expected = b"POST /test HTTP/1.1\r\ncontent-type: application/json\r\n\r\n{\"key\": \"value\"}";
        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_modify_headers_of_response_bytes() {
        let response_bytes = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 21\r\n\r\n{\"key\": \"value\"}";
        let headers_to_remove = vec!["Content-Type".to_string()];
        let headers_to_set = HashMap::new();
        let cookies_to_set= HashMap::new();
        let update_content_length = true;
        let modified_bytes = modify_headers_of_response_bytes(
            response_bytes,
            headers_to_remove,
            headers_to_set,
            cookies_to_set,
            update_content_length,
        );
        let expected = b"HTTP/1.1 200 OK\r\ncontent-length: 16\r\n\r\n{\"key\": \"value\"}";
        assert_eq!(modified_bytes, expected);
    }

    #[test]
    fn test_response_to_bytes() {
        let response = Response::builder()
            .status(200)
            .version(Version::HTTP_11)
            .header("Content-Type", "application/json")
            .body(vec![123, 34, 107, 101, 121, 34, 58, 32, 34, 118, 97, 108, 117, 101, 34, 125]) // {"key": "value"}
            .unwrap();
        let bytes = response_to_bytes(&response);
        let expected = b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"key\": \"value\"}";
        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_to_json_vec() {
        let value = vec!["test".to_string(), "value".to_string()];
        let json_vec = to_json_vec(&value);
        assert_eq!(json_vec, b"[\"test\",\"value\"]");
    }

    #[test]
    fn test_from_json_slice() {
        let json_slice = b"[\"test\",\"value\"]";
        let value: Vec<String> = from_json_slice(json_slice).unwrap();
        assert_eq!(value, vec!["test".to_string(), "value".to_string()]);
    }

    #[test]
    fn test_to_json_string() {
        let value = vec!["test".to_string(), "value".to_string()];
        let json_string = to_json_string(&value);
        assert_eq!(json_string, "[\"test\",\"value\"]");
    }

    #[test]
    fn test_from_json_string() {
        let json_string = "[\"test\",\"value\"]";
        let value: Vec<String> = from_json_string(json_string).unwrap();
        assert_eq!(value, vec!["test".to_string(), "value".to_string()]);
    }
}
