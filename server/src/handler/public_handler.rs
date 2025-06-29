use std::collections::HashMap;

use chrono::Utc;
use common::convert::{parse_request_bytes, request_to_bytes, modify_headers_of_response_bytes};
use common::net::{
    http_json_response_as_bytes,
    get_cookie_from_request,
    HttpReader,
    HttpResponse,
    TcpStreamTLS
};
use hex;
use rand::{self, Rng};
use sha2::{Sha256, Digest};
use http::{Request, StatusCode, Uri};
use tokio::net::TcpStream;
use common::data::dto::public_request::PublicRequest;
use common::{_info, _error};
use common::config::keys as config_keys;
use crate::service::cache_service::CacheService;
use crate::service::client_service::ClientService;
use crate::service::public_service::PublicService;

const CLIENT_ID_COOKIE_KEY: &str = "trabas_client_id";
const TUNNEL_ID_HEADER_KEY: &str = "trabas_tunnel_id";

pub async fn register_public_handler(
    stream: TcpStream, 
    client_service: ClientService, 
    public_service: PublicService, 
    cache_service: CacheService, 
    cache_client_id: bool,
    return_tunnel_id: bool
) {
    tokio::spawn(async move {
        let (read_stream, write_stream) = tokio::io::split(stream);
        public_handler(
            TcpStreamTLS::from_tcp(read_stream, write_stream), 
            client_service, 
            public_service, 
            cache_service, 
            cache_client_id, 
            return_tunnel_id).await;
    });
}

// handling public request up to receive a response
// TODO: implement error responses
async fn public_handler(
    mut stream: TcpStreamTLS, 
    client_service: ClientService, 
    public_service: PublicService, 
    cache_service: CacheService,
    cache_client_id: bool,
    return_tunenl_id: bool
) -> () {
    // read data as bytes
    let mut raw_request = Vec::new();
    if let Err(e) = HttpReader::from_tcp_stream(&mut stream).read(&mut raw_request, true).await {
        _error!("Error reading incoming request: {}", e);
        return;
    }

    _info!("New request has just been read.");

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

            _error!("{}", msg);
            stream.write_all(&response).await.unwrap();
            return;
        }   
    };

    // get client and transfer request at the same time
    let (request, client_id, path) = match get_client_id(request, cache_client_id) {
        Ok(value) => value,
        Err(msg) => {
            _error!("{}", msg);
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

    // chek whether client is active
    let client_id = match client_service.check_client_validity(client_id).await {
        Ok(value) => value,
        Err(msg) => {
            let response = match http_json_response_as_bytes(
            HttpResponse::new(false, msg.clone()), StatusCode::from_u16(400).unwrap()) {
                Ok(value) => value,
                Err(_) => {
                    return;
                } 
            };

            _error!("{}", msg);
            stream.write_all(&response).await.unwrap();
            return;
        }
    };

    raw_request = request_to_bytes(&request);

    let request_id = generate_request_id(client_id.clone());

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

    _info!("Public Request: `{}`, client: `{}`, path: [{}] `{}`", request_id.clone(), client_id.clone(), request_method.clone(), request_uri.clone());

    // check cache
    let cache_config = cache_service.get_cache_config(client_id.clone(), request_method.clone(), path.clone()).await;
    match cache_config {
        Ok(_) => {
            match cache_service.get_cache(client_id.clone(), request_uri.clone(), request_method.clone(), request_body.clone()).await {
                Ok(cached_response) => {
                    _info!("Public Request: {} processed [cache hit].", request_id);
        
                    // return the cached response to public client
                    stream.write_all(&cached_response).await.unwrap();
        
                    return
                },
                Err(msg) => { _error!("Error getting cache for request {}: {}", request_id.clone(), msg) } // ignore error
            }
        },
        Err(_) => {}
    }

    let public_request = PublicRequest {
        id: request_id.clone(),
        data: raw_request
    };

    // enqueue the request
    if let Err(e) = public_service.enqueue_request(client_id.clone(), public_request).await {
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

    _info!("Public Request: {} was enqueued.", request_id.clone());
    
    // wait for response
    let timeout = std::env::var(config_keys::CONFIG_KEY_SERVER_PUBLIC_REQUEST_TIMEOUT)
        .ok()
        .and_then(|val| val.parse::<u64>().ok())
        .unwrap_or(60); // default timeout is 60 seconds
    let res = match public_service.get_response(client_id.clone(), request_id.clone(), timeout).await {
        Ok(value) => value,
        Err(msg) => {
            _error!("{}", msg);
            let response = http_json_response_as_bytes(
                HttpResponse::new(false, msg), StatusCode::from_u16(400).unwrap()).unwrap();

            stream.write_all(&response).await.unwrap();
            return;
        }
    };

    // normalize headers
    let res = normalize_response_headers(
        res.data, 
        if cache_client_id { Some(client_id.clone()) } else { None },
        if return_tunenl_id { Some(res.tunnel_id) } else { None }
    );

    // write cache
    match cache_config {
        Ok(config) => {
            if let Err(msg) = cache_service.set_cache(client_id, request_uri, request_method, request_body, res.clone(), config).await {
                _error!("Error writing cache for request {}: {}", request_id.clone(), msg);
            }
        },
        Err(_) => {}
    }

    _info!("Public Request: {} processed.", request_id);

    // finally return the response to public client
    stream.write_all(&res).await.unwrap();
}

fn normalize_response_headers(res: Vec<u8>, to_cache_client_id: Option<String>, to_return_tunnel_id: Option<String>) -> Vec<u8> {
    let headers_to_remove = vec![
        "Transfer-Encoding".to_string(),
        "Content-Length".to_string()
    ];
    let mut headers_to_set = HashMap::new();
    if let Some(tunnel_id) = to_return_tunnel_id {
        headers_to_set.insert(TUNNEL_ID_HEADER_KEY.to_string(), tunnel_id);
    }

    let mut cookies_to_set = HashMap::new();
    if let Some(client_id) = to_cache_client_id {
        cookies_to_set.insert(CLIENT_ID_COOKIE_KEY.to_string(), client_id);
    }
    
    return modify_headers_of_response_bytes(&res, headers_to_remove, headers_to_set, cookies_to_set, true);
}

// this returns unique bytes representation of body with cleaned insignificant part such "boundary"
// in multipart type body
fn get_unique_body_as_bytes(req: Request<Vec<u8>>) -> Vec<u8> {
    // Clean body boundary if exists, usually for multipart body
    if let Some(content_type) = req.headers().get("Content-Type") {
        let content_type_str = content_type.to_str().unwrap_or("");
        if content_type_str.contains("multipart/") {
            if let Some(boundary) = content_type_str.split("boundary=").nth(1) {
                // for simplicity, we'll just remove the boundary strings from the body
                // without parsing the multipart content
                // fixing depencency issue with `multipart` crate
                let body = req.body();
                let boundary_marker = format!("--{}", boundary.trim_matches('"'));
                let body_str = String::from_utf8_lossy(body);
                
                // remove boundary markers and normalize the content
                let cleaned = body_str
                    .lines()
                    .filter(|line| !line.starts_with(&boundary_marker))
                    .filter(|line| !line.trim().is_empty())
                    .collect::<Vec<&str>>()
                    .join("\n");
                
                return cleaned.into_bytes();
            }
        }
    }

    req.body().clone()
}

// get client id from request path
// the rules of this tunneling tool is always rely on the client id
// it must be provided in the request path (as prefix) or as a query parameter.
// suppose:
// client id: client_12345
// target path: /api/v1/ping
// A. Prefix path with client id:
//    - the accessible path from public: /[client id]/[actual path] -> /client_12345/api/v1/ping
// B. Pass client id as query parameter:
//    - the accessible path from public: /api/v1/ping?trabas_client_id=client_12345
// TODO: add client connection validation and retrier until a certain count
fn get_client_id<T>(mut request: Request<T>, cache_client_id: bool) -> Result<(Request<T>, String, String), String> {
    let cached_client = match cache_client_id {
        true => get_cookie_from_request(&request, CLIENT_ID_COOKIE_KEY),
        false => None
    };
    let uri = request.uri().clone();
    let mut path = uri.path().to_string();
    let mut query = uri.query().unwrap_or("").to_string();
    
    // check client id from request params
    let mut client_id = query.split('&')
        .find(|param| param.starts_with(CLIENT_ID_COOKIE_KEY))
        .and_then(|param| param.split('=').nth(1))
        .map(|id| id.to_string());
    let check_prefix = client_id.is_none();
    if check_prefix {
        // fallback to prefix path
        let mut check_path = path.clone();
        check_path = if check_path.starts_with('/') {
            check_path.remove(0);
            check_path
        } else {
            check_path.to_string()
        };
        let path_split: Vec<String> = check_path.split('/').map(|word| word.to_owned()).collect();
        // get first path as client id
        let check_client_id = path_split[0].clone();
        if !check_client_id.is_empty() {
            client_id = Some(check_client_id);
        }
    }

    let client_id = client_id.unwrap_or_default().to_string();
    let cached_client_id = cached_client.unwrap_or_default().trim().to_string();;
    if client_id.is_empty() && cached_client_id.is_empty() {
        // there's no way to identify the client
        return Err(String::from("Client ID cannot be empty or invalid."));
    }

    // check whether we rely on cached_client_id with conditions
    // - client_id is empty
    // - client_id is same as cached_client_id
    // - or client_id is not provided explicitly in the query params (i.e. no `trabas_client_id` in the query)
    // then, let the path be as is
    if client_id.is_empty() || (!cached_client_id.is_empty() && (client_id != cached_client_id || check_prefix)) {
        return Ok((request, cached_client_id, path));
    }

    // remove related client_id prefix path or query from the request
    let mut parts = uri.into_parts();
    let new_path_and_query = if check_prefix {
        // remove client id from path
        let path_split: Vec<String> = path.split('/').map(|word| word.to_owned()).collect();
        path = format!("/{}", (&path_split[2..]).join("/"));
        Some(format!("{}{}", path, if query.is_empty() { query } else { format!("?{}", query)}).parse().unwrap())
    } else {
        // just update the query
        query = query.split('&')
            .filter(|param| !param.starts_with(CLIENT_ID_COOKIE_KEY))
            .collect::<Vec<&str>>()
            .join("&");
        Some(format!("{}{}", path, if query.is_empty() { query } else { format!("?{}", query)}).parse().unwrap())
    };
    parts.path_and_query = new_path_and_query;

    // set request uri with the new uri
    let new_uri = Uri::from_parts(parts).unwrap();
    *request.uri_mut() = new_uri;

    Ok((request, client_id, path))
}

fn generate_request_id(client_id: String) -> String {
    // combine client_id and timestamp epoch
    let timestamp = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    // add randomness
    let mut rng = rand::rng();
    let random_suffix: Vec<u8> = (0..10).map(|_| rng.random()).collect();
    // concatenate client_id, timestamp, and random value
    let input = format!("{}{}{:?}", client_id, timestamp, random_suffix);
    // hash the value with a SHA-256 hasher
    let hash = Sha256::digest(input.as_bytes());
    
    // convert the hashed value to a hex string
    hex::encode(hash)[..32].to_string()
}
