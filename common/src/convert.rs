use http::{Request, Response, Version};
use httparse;
use std::collections::HashMap;
use std::io::Write;
use serde::Serialize;
use serde::de::DeserializeOwned;

// parse bytes request data to instance of http::Request 
pub fn parse_request_bytes(request_bytes: &[u8]) -> Option<Request<Vec<u8>>> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    match req.parse(request_bytes) {
        Ok(status) if status.is_complete() => {
            let method = req.method.unwrap();
            let path = req.path.unwrap();
            let version = match req.version.unwrap() {
                1 => Version::HTTP_11,
                2 => Version::HTTP_2,
                _ => Version::HTTP_10,
            };

            let mut request_builder = Request::builder()
                .method(method)
                .uri(path)
                .version(version);

            for h in req.headers {
                request_builder = request_builder.header(h.name, h.value);
            }

            // extract the body part
            let header_length = status.unwrap();
            let body = request_bytes[header_length..].to_vec();

            request_builder.body(body).ok()
        }
        _ => None,
    }
}

// parse request instance to bytes
pub fn request_to_bytes(request: &Request<Vec<u8>>) -> Vec<u8> {
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

    // lastly the body
    let body_bytes = request.body();
    bytes.extend_from_slice(&body_bytes);
    bytes
}

// TOOD: if the parameter is too many, convert to struct based (?)
// manipulate http response headers
pub fn modify_headers_of_response_bytes(
    response_bytes: &[u8],
    mut headers_to_remove: Vec<String>,
    cookies_to_set: HashMap<String, String>,
    update_content_length: bool,
) -> Vec<u8> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut res = httparse::Response::new(&mut headers);

    // always remove content length, since will be updated in the end
    headers_to_remove.push(String::from("Content-Length"));

    // parse the response from bytes
    match res.parse(response_bytes) {
        Ok(status) if status.is_complete() => {
            let status_code = res.code.unwrap();
            let version = match res.version.unwrap() {
                1 => Version::HTTP_11,
                2 => Version::HTTP_2,
                _ => Version::HTTP_10,
            };
            let mut response_builder = Response::builder()
                .status(status_code)
                .version(version);

            // add headers except the one to be removed
            for header in res.headers.iter().filter(|h| !headers_to_remove.contains(&(h.name.to_string()))) {
                response_builder = response_builder.header(header.name, header.value);
            }

            // set cookies
            if !cookies_to_set.is_empty() {
                for (key, value) in cookies_to_set {
                    let cookie_header = format!("{}={}; Path=/; HttpOnly", key, value);
                    response_builder = response_builder.header("Set-Cookie", cookie_header);
                }
            }

            // extract the body
            let header_length = status.unwrap();
            let body = response_bytes[header_length..].to_vec();

            // add content len based on current size
            if update_content_length {
                let content_length = body.len().to_string();
                response_builder = response_builder.header("Content-Length", content_length.as_str());
            }

            match response_builder.body(body) {
                Ok(response) => {
                    // convert the response to bytes
                    let mut final_response_bytes = Vec::new();

                    // write the status line
                    let status_line = format!(
                        "{:?} {} {}\r\n",
                        version,
                        status_code,
                        res.reason.unwrap_or("OK")
                    );
                    final_response_bytes.extend_from_slice(status_line.as_bytes());

                    // write headers
                    for (name, value) in response.headers().iter() {
                        final_response_bytes.extend_from_slice(name.as_str().as_bytes());
                        final_response_bytes.extend_from_slice(b": ");
                        final_response_bytes.extend_from_slice(value.as_bytes());
                        final_response_bytes.extend_from_slice(b"\r\n");
                    }

                    // end headers section
                    final_response_bytes.extend_from_slice(b"\r\n");

                    // write body
                    final_response_bytes.extend_from_slice(&response.body());

                    final_response_bytes
                }
                Err(_) => response_bytes.to_vec(),
            }
        }
        _ => response_bytes.to_vec(),
    }
}

// parse response to bytes
// TODO: accept no string format
pub fn response_to_bytes(response: &Response<Vec<u8>>) -> Vec<u8> {
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

    // lastly the body
    let body_bytes = response.body();
    bytes.extend_from_slice(&body_bytes);
    
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

// parse any type to json string
pub fn to_json_string<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("Error serializing to JSON string: {}", e);
            String::new()
        }
    }
}

// parse json string to actual type
pub fn from_json_string<T: DeserializeOwned>(json_str: &str) -> Option<T> {
    match serde_json::from_str::<T>(json_str) {
        Ok(value) => Some(value),
        Err(e) => {
            eprintln!("Error deserializing from JSON string: {}", e);
            None
        }
    }
}
