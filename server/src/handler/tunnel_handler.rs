use common::convert::{from_json_slice, to_json_vec};
use common::data::dto::tunnel_ack::TunnelAck;
use common::net::{
    append_path_to_url, prepare_packet, read_bytes_from_mutexed_socket_for_internal, read_bytes_from_socket_for_internal, separate_packets, TcpStreamTLS, HEALTH_CHECK_PACKET_ACK
};
use common::{validate_signature, _error, _info};
use tokio::time::{sleep, Instant};
use std::sync::Arc;
use std::time::Duration;
use std::u64;
use tokio::sync::Mutex;
use common::config;
use common::string;
use common::data::dto::public_response::PublicResponse;
use common::data::dto::tunnel_client::TunnelClient;

use crate::config::ext_keys;
use crate::service::client_service::ClientService;
use crate::service::public_service::PublicService;
use crate::version::{get_server_version, get_min_client_version};

pub async fn register_tunnel_handler(mut read_stream: TcpStreamTLS, mut write_stream: TcpStreamTLS, client_service: ClientService, public_service: PublicService) -> () {
    let tunnel_id = string::generate_rand_id(32);
    
    _info!("Pending tunnel [{}] connection.", tunnel_id.clone());

    // register client ID
    let mut raw_response = Vec::new();
    const SOCKET_TIMEOUT_MILLIS: u64 = 5000; // 5 seconds timeout
    if let Err(e) = read_bytes_from_socket_for_internal(&mut read_stream, &mut raw_response, SOCKET_TIMEOUT_MILLIS).await {
        _error!("Error reading connection: {}", e);
        return;
    }

    let (packets, _) = separate_packets(raw_response);
    let raw_response = match packets.get(0) {
        Some(data) => data,
        None => {
            _error!("Error reading connection: empty data");
            return;
        }
    };

    _info!("Done reading connection.");
    let client: TunnelClient = match from_json_slice(&raw_response) {
        Some(value) => value,
        None => {
            let tunnel_ack = TunnelAck::fails(tunnel_id, "Invalid request".to_string());
            let packet = prepare_packet(to_json_vec(&tunnel_ack));
            write_stream.write_all(&packet).await.unwrap();
            _error!("{}", tunnel_ack.message);
            return;    
        }
    };

    // validate versions
    let version = get_server_version();
    let min_client_version = get_min_client_version();
    if !client.validate_version(version.clone(), min_client_version.clone()) {
        let tunnel_ack = TunnelAck::fails(
            tunnel_id,
            format!(
                "Version mismatch: Server version code = {} (required ≥ {}) | Client version code = {} (required ≥ {}).",
                version.clone(),
                client.min_sv_version,
                client.cl_version,
                min_client_version
            ),
        );
        let packet = prepare_packet(to_json_vec(&tunnel_ack));
        write_stream.write_all(&packet).await.unwrap();
        _error!("{}", tunnel_ack.message);
        return;
    }

    let client_id = client.id.clone();
    let client_mac = format!("{}_{}", client.id, client.alias_id);
    // validate connection before registering client
    if !validate_signature(client.signature.clone(), client_mac.clone()) {
        let tunnel_ack = TunnelAck::fails(
            tunnel_id,
            format!("Client Registration Denied. client_id: {}, signature: {}", client_id, client.signature),
        );
        let packet = prepare_packet(to_json_vec(&tunnel_ack));
        write_stream.write_all(&packet).await.unwrap();
        _error!("{}", tunnel_ack.message);
        return;
    }

    // acknowledge the successful handshake
    // public endpoints are returned by the server because server should control the mechanism
    // and might change it in the future
    let endpoint_prefix = std::env::var(config::keys::CONFIG_KEY_SERVER_PUBLIC_ENDPOINT).unwrap_or_default();
    let endpoint_prefix = append_path_to_url(&endpoint_prefix, "");
    let public_endpoints = vec![
        format!("{}{} or {}?{}={}", endpoint_prefix, &client.id, &endpoint_prefix, ext_keys::CLIENT_ID_COOKIE_KEY, &client.id),
        format!("{}{} or {}?{}={}", endpoint_prefix, &client.alias_id, &endpoint_prefix, ext_keys::CLIENT_ID_COOKIE_KEY, &client.alias_id),
    ];
    let tunnel_ack = TunnelAck::success(tunnel_id.clone(), client_mac, get_server_secret(), public_endpoints);
    let packet = prepare_packet(to_json_vec(&tunnel_ack));
    write_stream.write_all(&packet).await.unwrap();

    let msg = format!("Client Registration Successful. client_id: {}, signature: {}, tunnel_id: {}", client_id, client.signature, tunnel_id.clone());
    _info!("{}", msg);

    // sleep for 1.5 seconds to prevent race condition with healthcheck packet
    sleep(Duration::from_millis(1500)).await;

    client_service.register_client(client, tunnel_id.clone()).await.unwrap();

    // isolate stream and service inside Arc
    let read_stream_arc = Arc::new(Mutex::new(read_stream));
    let write_stream_arc = Arc::new(Mutex::new(write_stream));
    let client_service_arc1 = Arc::new(Mutex::new(client_service));
    let client_service_arc2 = client_service_arc1.clone();
    let client_service_arc3 = client_service_arc1.clone();
    let public_service_arc1 = Arc::new(Mutex::new(public_service));
    let public_service_arc2 = public_service_arc1.clone();

    // share handler stop state between sender and reciever
    let handler_stopped1 = Arc::new(Mutex::new(false));
    let handler_stopped2 = handler_stopped1.clone();
    let handler_stopped3 = handler_stopped1.clone();

    // tunnel count with the same client id
    let tunnel_cnt1 = Arc::new(Mutex::new(0));
    let tunnel_cnt2 = tunnel_cnt1.clone();

    // init tunnel count
    {
        let mut tunnel_cnt = tunnel_cnt1.lock().await;
        *tunnel_cnt = client_service_arc1.lock().await.get_tunnel_count(client_id.clone()).await;
    }

    // client ids for each handler
    let client_id1 = client_id.clone();
    let client_id2 = client_id.clone();
    let client_id3 = client_id.clone();

    // tunnel ids for each handler
    let tunnel_id1 = tunnel_id;
    let tunnel_id2 = tunnel_id1.clone();
    let tunnel_id3 = tunnel_id2.clone();

    // spawn handlers
    // to prevent deadlocks, any lock should be acquired
    // inside a minimal scope, why?
    // because, currently there's a potential deadlock where
    // the ordering of locks is opposite in the sender and receiver handlers
    // Sender: public_service -> stream
    // Receiver: stream -> public_service
    tokio::spawn(async move {
        tunnel_sender_handler(
            handler_stopped1, 
            tunnel_cnt1,
            write_stream_arc, 
            public_service_arc1,
            client_service_arc1, 
            client_id1, 
            tunnel_id1).await;
    });
    tokio::spawn(async move {
        tunnel_receiver_handler(
            handler_stopped2, 
            read_stream_arc, 
            public_service_arc2, 
            client_service_arc2, 
            client_id2, 
            tunnel_id2).await;
    });
    tokio::spawn(async move {
        check_client_validity_handler(
            handler_stopped3,
            tunnel_cnt2, 
            client_service_arc3, 
            client_id3, 
            tunnel_id3).await;
    });
}

fn validate_signature(signature: String, mac: String) -> bool {
    let secret = get_server_secret();
    validate_signature!(signature, mac, secret)
}

fn get_server_secret() -> String {
    std::env::var(config::keys::CONFIG_KEY_SERVER_SECRET).unwrap_or_default()
}

// Tunnel Connection
// To form a bidirectional TCP connection, both server and client must perform
// different type of operations respectively, for example:
//   T+0 Server Write
//   T+1 Client Read
//   or
//   T+0 Client Write
//   T+1 Server Read
// This behaviour does not allow these operations:
//   T+0 Server Read
//   T+1 Client Read
//   or
//   T+0 Server Write
//   T+1 Client Write
// In a single stream instance, that only makes dead lock on both sides and the connection tracibility is inobvious
// So, we need to keep the proper sequence like this simulation:
//   T ..-1  | +0  +1  | +2  +3  | +4  +5  | +6  +7 | +8..
//   Server  | w       |      r  |      r  |  w     |
//   Client  |      r  |  w      |  w      |      r |
//
// That's why we need to separate the stream into `reader` and `writer`
// that allows such behaviour as the simulation above
//
// TODO: optimize server-client connection
// Phase 0 (implemented): Full synchronous (decent for a few requests)
// Phase 1 (implemented): Separate stream writer and reader
// Phase 2 (might)      : Write data in chunks for all requests (This is also helpful for a large request).
//                        But, we need to manage it efficiently to avoid any overheads.
async fn tunnel_sender_handler(
    handler_stopped: Arc<Mutex<bool>>,
    tunnel_count: Arc<Mutex<i64>>,
    stream: Arc<Mutex<TcpStreamTLS>>, 
    public_service: Arc<Mutex<PublicService>>, 
    client_service: Arc<Mutex<ClientService>>, 
    client_id: String,
    tunnel_id: String,
) {
    _info!("Tunnel [{}] sender handler started.", tunnel_id.clone());

    let mut last_hc = Instant::now();
    const HC_INTERVAL: u64 = 30; // in seconds
    const IDLE_SLEEP: u64 = 50; // in milliseconds
    const MIN_IDLE_SLEEP: u64 = 5; // in milliseconds
    while !(*handler_stopped.lock().await) {
        // check current tunnel count
        let curr_tunnel_count = {
            let tunnel_count = tunnel_count.lock().await;
            *tunnel_count
        };

        // make sure of tunnel distribution by dynamically adjusting idle sleep
        // based on the current tunnel count
        // this should simulate Round-robin for multiple tunnels with a single client id
        // despite it's not a strict round-robin, it should be enough
        // TODO: when we deploy multiple server instances, the tunnel count
        // should be unique across the instances
        // need implementation for adding server instance id/key in the client registration
        let acquired_idle_sleep = if curr_tunnel_count > 1 {
            MIN_IDLE_SLEEP * (curr_tunnel_count as u64)
        } else {
            0
        };
        let not_acquired_idle_sleep = if curr_tunnel_count > 1 {
            MIN_IDLE_SLEEP
        } else {
            IDLE_SLEEP
        };
        
        // request from the queue
        let public_request_opt = {
            public_service.lock().await.dequeue_request(client_id.clone()).await.ok()
        };
        
        match public_request_opt {
            Some(public_request) => {
                _info!("Request [{}] was acquired by tunnel [{}]", public_request.id.clone(), tunnel_id.clone());
                
                // send request to client service
                let bytes_req = prepare_packet(to_json_vec(&public_request.clone()));
                let write_res = {
                    stream.lock().await.write_all(&bytes_req).await
                };
                
                match write_res {
                    Ok(_) => {
                        _info!("Request: {} was sent to client: {}.", public_request.id, client_id.clone());
                        // reset health check here
                        last_hc = Instant::now();
                    },
                    Err(err) => {
                        _error!("Error sending request [{}] to client [{}]: {}", public_request.id, client_id, err);
                        break;
                    }
                }

                // even we successfully acquired a request
                // perform idle sleep anyway, give other tunnels a chance to process
                // (in case of multiple tunnels with a single client id)
                sleep(Duration::from_millis(acquired_idle_sleep)).await;
            },
            None => {
                if last_hc.elapsed() > Duration::from_secs(HC_INTERVAL) {
                    _info!("Sending health check to client service [{}] after {} seconds idle...", client_id, HC_INTERVAL);
                    let hc = prepare_packet(Vec::from(String::from(HEALTH_CHECK_PACKET_ACK).as_bytes()));
                    let hc_res = {
                        stream.lock().await.write_all(&hc).await
                    };
                    
                    if hc_res.is_err() {
                        break;
                    }

                    last_hc = Instant::now();
                }
                
                sleep(Duration::from_millis(not_acquired_idle_sleep)).await;
            }
        }
    }

    let client_dc = {
        let mut stopped = handler_stopped.lock().await;
        if !*stopped {
            *stopped = true;
            true 
        } else {
            false
        }
    };

    if client_dc {
        // disconnect client
        if let Err(e) = client_service.lock().await.disconnect_client(client_id.clone(), tunnel_id.clone()).await {
            _error!("Error disconnecting client [{}] tunnel [{}]: {}", client_id, tunnel_id, e);
        } else {
            _info!("Client Disconnected. client_id: {}, tunnel_id: {}.", client_id, tunnel_id.clone());
        }
    }

    _info!("Tunnel [{}] sender handler stopped.", tunnel_id);
}

async fn tunnel_receiver_handler(
    handler_stopped: Arc<Mutex<bool>>,
    stream: Arc<Mutex<TcpStreamTLS>>, 
    public_service: Arc<Mutex<PublicService>>, 
    client_service: Arc<Mutex<ClientService>>, 
    client_id: String,
    tunnel_id: String,
) {
    _info!("Tunnel [{}] receiver handler started.", tunnel_id.clone());

    let mut last_received = Instant::now();
    const TIMEOUT: u64 = 3; // in seconds
    const IDLE_SLEEP: u64 = 50; // in milliseconds
    while !(*handler_stopped.lock().await) {
        // get latest response from stream
        let mut raw_response = Vec::new();
        if let Err(e) = read_bytes_from_mutexed_socket_for_internal(stream.clone(), &mut raw_response, u64::MAX).await {
            _error!("{}", e);
            break;
        }

        // empty response
        if raw_response.len() == 0 {
            if last_received.elapsed() > Duration::from_secs(TIMEOUT) {
                _info!("Connection hung up for {} seconds, stopping receiver handler...", TIMEOUT);
                break;
            }
            // idle sleep
            sleep(Duration::from_millis(IDLE_SLEEP)).await;
            continue;
        }

        last_received = Instant::now();

        let (packets, contains_health_check) = separate_packets(raw_response);
        if contains_health_check {
            _info!("Received health check packet from client service [{}].", client_id);
        }
        for packet in packets {
            // enqueue Public Response
            let mut response: PublicResponse = match from_json_slice(&packet) {
                Some(value) => { value },
                None => {
                    break;
                }
            };

            // assign tunnel_id to response
            response.tunnel_id = tunnel_id.clone();
            let assign_res = {
                public_service.lock().await.assign_response(client_id.clone(), response.clone()).await
            };
            
            if let Err(msg) = assign_res {
                _error!("{}", msg);
                continue;
            }

            _info!("Response received by tunnel [{}] for request: {}.", tunnel_id.clone(), response.request_id);
        }
    }

    let client_dc = {
        let mut stopped = handler_stopped.lock().await;
        if !*stopped {
            *stopped = true;
            true
        } else {
            false 
        }
    };

    if client_dc {
        // disconnect client
        if let Err(e) = client_service.lock().await.disconnect_client(client_id.clone(), tunnel_id.clone()).await {
            _error!("Error disconnecting client [{}] tunnel [{}]: {}", client_id, tunnel_id, e);
        } else {
            _info!("Client Disconnected. client_id: {}, tunnel_id: {}", client_id, tunnel_id.clone());
        }
    }

    _info!("Tunnel [{}] receiver handler stopped.", tunnel_id);
}

// to make sure of the client validity
// where this is required in the public request
// if it's invalid, just break the tunnel
// let the client restart itself
// TODO: might write last checked timestamp for each tunnel id
async fn check_client_validity_handler(
    handler_stopped: Arc<Mutex<bool>>,
    tunnel_count: Arc<Mutex<i64>>,
    client_service: Arc<Mutex<ClientService>>,
    client_id: String,
    tunnel_id: String,
) {
    _info!("Client [{}] validity check for Tunnel [{}] handler started.", client_id.clone(), tunnel_id.clone());
    const IDLE_SLEEP: u64 = 1000; // in milliseconds
    let mut of_invalid = false;
    while !(*handler_stopped.lock().await) {
        let curr_tunnel_count = {
            // check from client service
            client_service.lock().await.get_tunnel_count(client_id.clone()).await
        };
        if curr_tunnel_count <= 0 {
            of_invalid = true;
            break;
        }
        // update tunnel count
        {
            let mut tunnel_count = tunnel_count.lock().await;
            *tunnel_count = curr_tunnel_count;
        }
        // idle sleep
        sleep(Duration::from_millis(IDLE_SLEEP)).await;
    }

    {
        let mut stopped = handler_stopped.lock().await;
        *stopped = true;
    }
    
    if of_invalid {
        // we don't need to disconnect the client here
        // since, it's already disconnected
        _error!("Client [{}] invalid or inactive. Stopping Tunnel [{}]...", client_id, tunnel_id);
    }
    
    _info!("Client [{}] validity check for Tunnel [{}] handler stopped.", client_id.clone(), tunnel_id.clone());
}
