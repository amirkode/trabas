use http::{Request, Response, Version};
use httparse;
use std::io::Write;
use serde::Serialize;
use serde::de::DeserializeOwned;

// parse bytes request data to instance of http::Request 
pub fn parse_request_bytes(request_bytes: &[u8]) -> Option<Request<()>> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    match req.parse(request_bytes) {
        Ok(status) if status.is_complete() => {
            let method = req.method.unwrap();
            let path = req.path.unwrap();
            let version = match req.version.unwrap() {
                1 => Version::HTTP_11,
                2 => Version::HTTP_2,
                _ => Version::HTTP_10
            };

            let mut request_builder = Request::builder()
                .method(method)
                .uri(path)
                .version(version);

            for h in req.headers {
                request_builder = request_builder.header(h.name, h.value);
            }

            request_builder.body(()).ok()
        }
        _ => None
    }
}

// parse request instance to bytes
pub fn request_to_bytes<T>(request: &Request<T>) -> Vec<u8> {
    let mut bytes = Vec::new();
    
    let method = request.method().as_str();
    let uri = request.uri().to_string();
    let version = match request.version() {
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        _ => "HTTP/1.0"
    };
    
    write!(bytes, "{} {} {}\r\n", method, uri, version).unwrap();
    
    for (name, value) in request.headers() {
        write!(bytes, "{}: {}\r\n", name, value.to_str().unwrap()).unwrap();
    }
    
    write!(bytes, "\r\n").unwrap();
    
    bytes
}

// parse response to bytes
// TODO: accept no string format
pub fn response_to_bytes(response: &Response<String>) -> Vec<u8> {
    let mut bytes = Vec::new();
    
    let version = match response.version() {
        Version::HTTP_10 => "HTTP/1.0",
        Version::HTTP_11 => "HTTP/1.1",
        Version::HTTP_2 => "HTTP/2.0",
        _ => "HTTP/1.0",
    };
    
    let status = response.status();
    write!(bytes, "{} {} {}\r\n", version, status.as_u16(), status.canonical_reason().unwrap_or("")).unwrap();
    
    for (name, value) in response.headers() {
        write!(bytes, "{}: {}\r\n", name, value.to_str().unwrap()).unwrap();
    }
    
    write!(bytes, "\r\n").unwrap();
    
    bytes
}

// parse any type to json vec
pub fn to_json_vec<T: Serialize>(value: &T) -> Vec<u8> {
    match serde_json::to_vec(value) {
        Ok(vec) => vec,
        Err(e) => {
            eprintln!("Error serializing to JSON: {}", e);
            Vec::new()
        }
    }
}

// parse json slice to actual type
pub fn from_json_slice<T: DeserializeOwned>(slice: &[u8]) -> Option<T> {
    match serde_json::from_slice::<T>(slice) {
        Ok(value) => Some(value),
        Err(e) => {
            eprintln!("Error deserializing from JSON: {}", e);
            None
        }
    }
}
