// parse bytes request data to instance of http::Request 
#[macro_export]
macro_rules! parse_request_bytes {
    ($request_bytes:expr) => {{
        use http::{Request, Version};

        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse($request_bytes) {
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

                match request_builder.body(()) {
                    Ok(request) => Some(request),
                    Err(_) => None
                }
            }
            _ => None
        }
    }}
}

#[macro_export]
macro_rules! request_to_bytes {
    ($request:expr) => {{
        use http::Version;
        use std::io::Write;

        let mut bytes = Vec::new();
        
        // construct with the request line
        let method = $request.method().as_str();
        let uri = $request.uri().to_string();
        let version = match $request.version() {
            Version::HTTP_10 => "HTTP/1.0",
            Version::HTTP_11 => "HTTP/1.1",
            Version::HTTP_2 => "HTTP/2.0",
            _ => "HTTP/1.0"
        };
        
        write!(bytes, "{} {} {}\r\n", method, uri, version).unwrap();
        
        // append headers
        for (name, value) in $request.headers() {
            write!(bytes, "{}: {}\r\n", name, value.to_str().unwrap()).unwrap();
        }
        
        // end headers with a blank line
        write!(bytes, "\r\n").unwrap();
        
        bytes
    }}
}

// parse any type to json vec
#[macro_export]
macro_rules! to_json_vec {
    ($value:expr) => {
        match serde_json::to_vec(&$value) {
            Ok(vec) => vec,
            Err(e) => {
                eprintln!("Error serializing to JSON: {}", e);
                Vec::new()
            }
        }
    };
}

// parse json slice to actual type
#[macro_export]
macro_rules! from_json_slice {
    ($slice:expr, $type:ty) => {
        match serde_json::from_slice::<$type>($slice) {
            Ok(value) => Some(value),
            Err(e) => {
                eprintln!("Error deserializing from JSON: {}", e);
                None
            }
        }
    };
}
