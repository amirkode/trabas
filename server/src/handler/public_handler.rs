use chrono::Utc;
use common::convert::{parse_request_bytes, request_to_bytes, response_to_bytes};
use hex;
use log::{error, info};
use sha2::{Sha256, Digest};
use http::{Request, Uri};
use http::{Response, Version};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use common::data::dto::public_request::PublicRequest;
use crate::service::public_service::PublicService;

pub async fn register_public_handler(mut stream: TcpStream, service: PublicService) {
    tokio::spawn(async move {
        public_handler(stream, service).await;
    });
}

// handling public request up to receive a response
// TODO: implement error responses
async fn public_handler(mut stream: TcpStream, service: PublicService) -> () {
    // read data as bytes
    let mut buffer = [0; 1024];
    let mut raw_request = Vec::new();
    loop {
        let n = stream.read(&mut buffer).await.unwrap();
        raw_request.extend_from_slice(&buffer[..n]);
        
        // until the end of the headers
        if raw_request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    // stream.read(&mut raw_request).await.unwrap();

    info!("New request has just been read");

    // parse the raw request
    let request = match parse_request_bytes(&raw_request) {
        Some(value) => value,
        None => {
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\noeieiei!";

            stream.write_all(response.as_bytes()).await.unwrap();
            error!("Parsing on empty request");
            return;
        }   
    };
    // get client and transfor request at the same time
    let (request, client_id) = match get_client_id(request) {
        Ok(value) => value,
        Err(msg) => {
            error!("{}", msg);
            let response = Response::builder()
            .version(Version::HTTP_11)
            .status(200)
            .header("Content-Type", "text/plain")
            .body(String::from("thequickborwnfojumpsover"))
            .unwrap();

            stream.write_all(&response_to_bytes(&response)).await.unwrap();
            // stream.shutdown().await.unwrap();
            // stream.write_all(msg.as_bytes()).await.unwrap();
            return;
        }
    };
    raw_request = request_to_bytes(&request);

    let request_id = genereate_request_id(client_id.clone());
    let public_request = PublicRequest {
        client_id,
        id: request_id.clone(),
        data: raw_request
    };

    // enqueue the request to redis
    service.enqueue_request(public_request).await.unwrap();

    info!("Public Request: {} was enqueued.", request_id.clone());
    
    // wait for response
    let timeout = 30u64; // time out in seconds
    let res = service.get_response(request_id.clone(), timeout).await.unwrap();

    info!("Public Request: {} processed.", request_id);

    // finally return the response to public client
    stream.write_all(&(res.data)).await.unwrap();
}

// get client id from request path
// the rules of this tunneling tool is always prepend path with client_id
// suppose:
// client id: client_12345
// actual path: /api/v1/ping
// so, the accessible path from public:
// /[client id]/[actual path] -> /client_12345/api/v1/ping
// TODO: add client connection validation and retrier until a certain count
fn get_client_id<T>(mut request: Request<T>) -> Result<(Request<T>, String), String> {
    let uri = request.uri().clone();
    let mut path = uri.path().to_string();
    let path = if path.starts_with('/') {
        path.remove(0);
        path
    } else {
        path.to_string()
    };
    let path_split: Vec<String> = path.split('/').map(|word| word.to_owned()).collect();
    // get first path as client id
    let client_id = path_split[0].clone();
    if client_id.is_empty() {
        return Err(String::from("Client ID cannot be empty or invalid."))
    }
    // remove first path from the split
    let new_path = format!("/{}", (&path_split[1..]).join("/"));
    // update query
    let mut parts = uri.into_parts();
    let new_path_and_query = match parts.path_and_query {
        Some(pq) => {
            let query = pq.query().map(|q| format!("?{}", q)).unwrap_or_default();
            Some(format!("{}{}", new_path, query).parse().unwrap())
        },
        None => Some(new_path.parse().unwrap()),
    };
    parts.path_and_query = new_path_and_query;

    // set request uri with the new uri
    let new_uri = Uri::from_parts(parts).unwrap();
    *request.uri_mut() = new_uri;

    Ok((request, client_id))
}

fn genereate_request_id(client_id: String) -> String {
    // combine client_id and timestamp epoch
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap().to_string();
    let input = format!("{}{}", client_id, timestamp);
    
    // hash the value with a SHA-256 hasher
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    // convert the hashed value to a hex string
    let hex_result = hex::encode(result);
    // the result is fixed to 32 chars
    let id = &hex_result[..32];
    
    id.to_string()
}