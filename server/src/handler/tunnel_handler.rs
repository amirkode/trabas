
use common::convert::{from_json_slice, to_json_vec};
use common::data::dto::tunnel_ack::TunnelAck;
use common::net::{
    append_path_to_url, prepare_packet, read_bytes_from_mutexed_socket_for_internal, read_bytes_from_socket_for_internal, separate_packets, TcpStreamTLS, HEALTH_CHECK_PACKET_ACK
};
use common::validate_signature;
use log::{error, info};
use tokio::net::TcpStream;
use tokio::time::sleep;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use common::data::dto::public_response::PublicResponse;
use common::data::dto::tunnel_client::TunnelClient;
use crate::config;
use crate::service::client_service::ClientService;
use crate::service::public_service::PublicService;

pub async fn register_tunnel_handler(stream: TcpStream, client_service: ClientService, public_service: PublicService) -> () {
    info!("Pending connection");
    let (read_stream, write_stream) = tokio::io::split(stream);
    let mut read_stream = TcpStreamTLS::from_tcp_read(read_stream);
    let mut write_stream = TcpStreamTLS::from_tcp_write(write_stream);
    // register client ID
    let mut raw_response = Vec::new();
    if let Err(e) = read_bytes_from_socket_for_internal(&mut read_stream, &mut raw_response).await {
        error!("Error reading connection: {}", e);
        return;
    }

    let packets = separate_packets(raw_response);
    let raw_response = match packets.get(0) {
        Some(data) => data,
        None => {
            error!("Error reading connection: empty data");
            return;
        }
    };

    info!("Done reading connection");
    let client: TunnelClient = match from_json_slice(&raw_response) {
        Some(value) => value,
        None => {
            let tunnel_ack = TunnelAck::new(false, format!("Invalid request"), vec![]);
            let packet = prepare_packet(to_json_vec(&tunnel_ack));
            write_stream.write_all(&packet).await.unwrap();
            error!("{}", tunnel_ack.message);
            return;    
        }
    };
    let client_id = client.id.clone();
    // validate connection before registering client
    if !validate_connection(client.signature.clone(), client.id.clone()) {
        let tunnel_ack = TunnelAck::new(
            false, 
            format!("Client Registration Denied. client_id: {}, signature: {}", client_id, client.signature), 
            vec![]);
        let packet = prepare_packet(to_json_vec(&tunnel_ack));
        write_stream.write_all(&packet).await.unwrap();
        error!("{}", tunnel_ack.message);
        return;
    }

    // acknowledge the successful handshake
    // public endpoints are returned by the server because server should control the mechanism
    // and might change it in the future
    let endpoint_prefix = std::env::var(config::CONFIG_KEY_SERVER_PUBLIC_ENDPOINT).unwrap_or_default();
    let public_endpoints = vec![append_path_to_url(&endpoint_prefix, &client.id), append_path_to_url(&endpoint_prefix, &client.alias_id)];
    let tunnel_ack = TunnelAck::new(
        true, 
        format!("ok"), 
        public_endpoints);
    let packet = prepare_packet(to_json_vec(&tunnel_ack));
    write_stream.write_all(&packet).await.unwrap();

    let msg = format!("Client Registration Successful. client_id: {}, signature: {}", client_id, client.signature);
    info!("{}", msg);

    // sleep for 1.5 seconds to prevent race condition with healthcheck packet
    sleep(Duration::from_millis(1500)).await;

    client_service.register_client(client).await.unwrap();

    // isolate stream and service inside Arc
    let read_stream_arc = Arc::new(Mutex::new(read_stream));
    let write_stream_arc = Arc::new(Mutex::new(write_stream));
    let client_service_arc1 = Arc::new(Mutex::new(client_service));
    let client_service_arc2 = client_service_arc1.clone();
    let public_service_arc1 = Arc::new(Mutex::new(public_service));
    let public_service_arc2 = public_service_arc1.clone();

    // share handler stop state between sender and reciever
    let handler_stopped1 = Arc::new(Mutex::new(false));
    let handler_stopped2 = handler_stopped1.clone();

    // client ids for each handler
    let client_id1 = client_id.clone();
    let client_id2 = client_id.clone();

    // spawn handlers
    tokio::spawn(async move {
        tunnel_sender_handler(handler_stopped1, write_stream_arc, public_service_arc1, client_service_arc1, client_id1).await;
    });
    tokio::spawn(async move {
        tunnel_receiver_handler(handler_stopped2, read_stream_arc, public_service_arc2, client_service_arc2, client_id2).await;
    });
}

fn validate_connection(signature: String, client_id: String) -> bool {
    let secret = std::env::var(config::CONFIG_KEY_SERVER_SECRET).unwrap_or_default();
    validate_signature!(signature, client_id, secret)
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
    stream: Arc<Mutex<TcpStreamTLS>>, 
    public_service: Arc<Mutex<PublicService>>, 
    client_service: Arc<Mutex<ClientService>>, 
    client_id: String,
) {
    info!("Tunnel sender handler started.");
    
    let mut skip = 0;
    while !(*handler_stopped.lock().await) {
        // request from the queue
        match public_service.lock().await.dequeue_request(client_id.clone()).await {
            Ok(public_request) => {
                info!("Request was found: {}", public_request.id.clone());
                
                // send request to client service
                let bytes_req = prepare_packet(to_json_vec(&public_request.clone()));
                let _ = match stream.lock().await.write_all(&bytes_req).await {
                    Ok(ok) => ok,
                    Err(err) => {
                        format!("{}", err);
                        break;
                    }
                };

                info!("Request: {} was sent to client: {}.", public_request.id, client_id.clone());
            },
            Err(_) => {
                skip += 1;
                // every 20k skips send health check
                if skip == 20000 {
                    let hc = prepare_packet(Vec::from(String::from(HEALTH_CHECK_PACKET_ACK).as_bytes()));
                    if let Err(_) = stream.lock().await.write_all(&hc).await {
                        break;
                    }
                    // sleep for 100 ms
                    sleep(Duration::from_millis(100)).await;
                    skip = 0;
                }
            }
        }
    }

    if !(*handler_stopped.lock().await) {
        // disconnect client
        client_service.lock().await.disconnect_client(client_id.clone()).await.unwrap();
        info!("Client Disconnected. client_id: {}", client_id);
    }

    // update handler stop state
    (*handler_stopped.lock().await) = true;

    info!("Tunnel sender handler stopped.");
}

async fn tunnel_receiver_handler(
    handler_stopped: Arc<Mutex<bool>>,
    stream: Arc<Mutex<TcpStreamTLS>>, 
    public_service: Arc<Mutex<PublicService>>, 
    client_service: Arc<Mutex<ClientService>>, 
    client_id: String
) {
    info!("Tunnel receiver handler started.");

    while !(*handler_stopped.lock().await) {
        // get latest response from stream
        let mut raw_response = Vec::new();
        if let Err(e) = read_bytes_from_mutexed_socket_for_internal(stream.clone(), &mut raw_response).await {
            error!("{}", e);
            break;
        }

        // empty response
        if raw_response.len() == 0 {
            continue;
        }

        let packets = separate_packets(raw_response);
        for packet in packets {
            // enqueue Public Response
            let response: PublicResponse = match from_json_slice(&packet) {
                Some(value) => { value },
                None => {
                    break;
                }
            };

            if let Err(msg) = public_service.lock().await.assign_response(client_id.clone(), response.clone()).await {
                error!("{}", msg);
                continue;
            }

            info!("Response received for request: {}.", response.request_id);
        }
    }

    if !(*handler_stopped.lock().await) {
        // disconnect client
        client_service.lock().await.disconnect_client(client_id.clone()).await.unwrap();
        info!("Client Disconnected. client_id: {}", client_id);
    }

    // update handler stop state
    (*handler_stopped.lock().await) = true;

    info!("Tunnel receiver handler stopped.");
}
