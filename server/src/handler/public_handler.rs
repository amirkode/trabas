use std::io::{Cursor, Read};

use chrono::Utc;
use common::convert::{parse_request_bytes, request_to_bytes};
use common::net::{http_json_response_as_bytes, read_bytes_from_socket, HttpResponse, TcpStreamTLS};
use hex;
use log::{error, info};
use multipart::server::Multipart;
use sha2::{Sha256, Digest};
use http::{Request, StatusCode, Uri};
use tokio::net::TcpStream;
use common::data::dto::public_request::PublicRequest;
use crate::service::cache_service::CacheService;
use crate::service::public_service::PublicService;

pub async fn register_public_handler(stream: TcpStream, service: PublicService, cache_service: CacheService) {
    tokio::spawn(async move {
        public_handler(TcpStreamTLS::from_tcp(stream), service, cache_service).await;
    });
}

// handling public request up to receive a response
// TODO: implement error responses
async fn public_handler(mut stream: TcpStreamTLS, service: PublicService, cache_service: CacheService) -> () {
    info!("Tunnel handler started.");
    // read data as bytes
    let mut raw_request = Vec::new();
    if let Err(e) = read_bytes_from_socket(&mut stream, &mut raw_request).await {
        error!("{}", e);
        return;
    }

    info!("New request has just been read");

    // parse the raw request
    let request = match parse_request_bytes(&raw_request) {
        Some(value) => value,
        None => {
            let msg = "Parsing on empty request";
            let response = match http_json_response_as_bytes(
            HttpResponse::new(false, String::from(msg)), StatusCode::from_u16(400).unwrap()) {
                Ok(value) => value,
                Err(_) => {
                    return;
                } 
            };

            error!("{}", msg);
            stream.write_all(&response).await.unwrap();
            return;
        }   
    };
    // get client and transfer request at the same time
    let (request, client_id, path) = match get_client_id(request) {
        Ok(value) => value,
        Err(msg) => {
            error!("{}", msg);
            let response = match http_json_response_as_bytes(
            HttpResponse::new(false, msg), StatusCode::from_u16(400).unwrap()) {
                Ok(value) => value,
                Err(_) => {
                    return;
                } 
            };

            stream.write_all(&response).await.unwrap();
            // stream.shutdown().await.unwrap();
            // stream.write_all(msg.as_bytes()).await.unwrap();
            return;
        }
    };
    raw_request = request_to_bytes(&request);

    let request_id = genereate_request_id(client_id.clone());

    // check whether a cache of the request is available
    // TODO: Uniqueness up to header values (i.e: cookies) might make the cache ineffective
    //       even if the actual request share same input to the underlying client.
    //       That's why the unique factors are only request URI and Body.
    //       at least in my case (author) is still reliable, the request was sent to the tunnel (trabas) with no significant header values.
    //       If we want to include the headers to uniqueness of the request,
    //       might find a better approach to cover all cases. i.e: excluding serveral keys from header
    let request_uri = request.uri().to_string();
    let request_method = String::from(request.method().as_str());
    let request_body = get_unique_body_as_bytes(request.clone());
    let cache_config = cache_service.get_cache_config(client_id.clone(), request_method.clone(), path.clone()).await;

    match cache_config.clone() {
        Ok(_) => {
            match cache_service.get_cache(client_id.clone(), request_uri.clone(), request_method.clone(), request_body.clone()).await {
                Ok(cached_response) => {
                    info!("Public Request: {} processed.", request_id);
        
                    // return the cached response to public client
                    stream.write_all(&cached_response).await.unwrap();
        
                    return
                },
                Err(msg) => { error!("Error getting cache for request {}: {}", request_id.clone(), msg) } // ignore error
            }
        },
        Err(_) => {}
    }

    let public_request = PublicRequest {
        client_id: client_id.clone(),
        id: request_id.clone(),
        data: raw_request
    };

    // enqueue the request to redis
    if let Err(e) = service.enqueue_request(public_request).await {
        let response = match http_json_response_as_bytes(
        HttpResponse::new(false, e), StatusCode::from_u16(503).unwrap()) {
            Ok(value) => value,
            Err(_) => {
                return;
            } 
        };

        stream.write_all(&response).await.unwrap();
        return;
    };

    info!("Public Request: {} was enqueued.", request_id.clone());
    
    // wait for response
    let timeout = 30u64; // time out in 30 seconds
    let res = match service.get_response(client_id.clone(), request_id.clone(), timeout).await {
        Ok(value) => value,
        Err(msg) => {
            error!("{}", msg);
            let response = http_json_response_as_bytes(
                HttpResponse::new(false, msg), StatusCode::from_u16(400).unwrap()).unwrap();

            stream.write_all(&response).await.unwrap();
            return;
        }
    };

    match cache_config {
        Ok(config) => {
            if let Err(msg) = cache_service.set_cache(client_id, request_uri, request_method, request_body, res.data.clone(), config).await {
                error!("Error writing cache for request {}: {}", request_id.clone(), msg);
            }
        },
        Err(_) => {}
    }

    info!("Public Request: {} processed.", request_id);

    // finally return the response to public client
    stream.write_all(&(res.data)).await.unwrap();
}

// this returns unique bytes representation of body with cleaned insignificant part such "boundary"
// in multipart type body
fn get_unique_body_as_bytes(req: Request<Vec<u8>>) -> Vec<u8> {
    // Clean body boundary if exists, usually for multipart body
    if let Some(content_type) = req.headers().get("Content-Type") {
        let content_type_str = content_type.to_str().unwrap();
        if let Some(boundary) = content_type_str.split("boundary=").nth(1) {
            // let boundary = format!("--{}", boundary);
            let mut multipart = Multipart::with_body(Cursor::new(req.body().to_owned()), boundary);

            // reformat data
            let mut cleaned_body: Vec<String> = Vec::new();
            // while let Some(mut field) = multipart.read_entry().unwrap()
            while let Ok(Some(mut field)) = multipart.read_entry() {
                let field_name = field.headers.name.to_string();
                let mut field_data = String::new();
                let content_type = field.headers.content_type.clone();

                if let Ok(_) = field.data.read_to_string(&mut field_data) {
                    // return the cleaned body
                    cleaned_body.push(format!("Content-Type: {:?}, Field: {:?}, Text Data: {:?}", content_type, field_name, field_data));
                } else {
                    let mut buffer = vec![];
                    field.data.read_to_end(&mut buffer).unwrap();
                    cleaned_body.push(format!("Content-Type: {:?}, Field: {:?}, Binary Data: {:?}", content_type, field_name, buffer));
                }
            }

            return cleaned_body.into_iter()
                .flat_map(|s| s.into_bytes())
                .collect()
        }
    }

    req.body().clone()
}

// get client id from request path
// the rules of this tunneling tool is always prepend path with client_id
// suppose:
// client id: client_12345
// actual path: /api/v1/ping
// so, the accessible path from public:
// /[client id]/[actual path] -> /client_12345/api/v1/ping
// TODO: add client connection validation and retrier until a certain count
fn get_client_id<T>(mut request: Request<T>) -> Result<(Request<T>, String, String), String> {
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

    Ok((request, client_id, new_path))
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
