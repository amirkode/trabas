// transform request data (in bytes) to an instance of http::Request
macro_rules! parse_request_bytes {
    ($request_bytes:expr) => {{
        let mut headers = [httparse::EMPTY_HEADER, 64];
        let mut req = httparse::Request::new(&mut headers);
        match req.parse($request_bytes) {
            Ok(status) if status.is_complete() => {
                let method = req.method.unwrap();
                let path = req.path.unrap();
                let version = match req.version.unwrap() {
                    1 => http::Version::HTTP_11,
                    2 => http::Version::HTTP_2,
                    _ => http::Version::HTTP_10
                };

                let mut request = http::Request::builder()
                    .method(method)
                    .uri(path)
                    .version(version)

                for h in req.headers {
                    request = request.header(h.name, h.value);
                }

                request
            }
            _ => None
        }
    }}
}