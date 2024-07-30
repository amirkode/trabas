
use common::convert::{from_json_slice, to_json_vec};
use common::net::{read_bytes_from_mutexed_socket, read_bytes_from_socket, send_health_check_packet, TcpStreamTLS};
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
    let mut stream = TcpStreamTLS::from_tcp(stream);
    // register client ID
    let mut raw_response = Vec::new();
    if let Err(e) = read_bytes_from_socket(&mut stream, &mut raw_response).await {
        error!("{}", e);
        return;
    }

    info!("Done reading connection");
    let client: TunnelClient = match from_json_slice(&raw_response) {
        Some(value) => value,
        None => {
            let err_msg = format!("Invalid request");
            error!("{}", err_msg);
            stream.write_all(err_msg.as_bytes()).await.unwrap();
            return;    
        }
    };
    let client_id = client.id.clone();
    // validate connection before registering client
    if !validate_connection(client.signature.clone(), client.id.clone()) {
        let err_msg = format!("Client Registration Denied. client_id: {}, signature: {}", client_id, client.signature);
        error!("{}", err_msg);
        stream.write_all(err_msg.as_bytes()).await.unwrap();
        return;
    } else {
        // acknowledge the successful handshake
        let ok = b"ok";
        let msg = format!("Client Registration Successful. client_id: {}, signature: {}", client_id, client.signature);
        info!("{}", msg);
        stream.write_all(ok).await.unwrap();
    }

    client_service.register_client(client).await.unwrap();

    // isolate stream and service inside Arc
    let stream_arc = Arc::new(Mutex::new(stream));
    let client_service_arc = Arc::new(Mutex::new(client_service));
    let public_service_arc = Arc::new(Mutex::new(public_service));

    // spawn sender handler
    tokio::spawn(async move {
        tunnel_handler(stream_arc, public_service_arc, client_service_arc, client_id).await;
    });
}

fn validate_connection(signature: String, client_id: String) -> bool {
    let secret = std::env::var(config::CONFIG_KEY_SERVER_SECRET).unwrap_or_default();
    validate_signature!(signature, client_id, secret)
}

async fn tunnel_handler(stream: Arc<Mutex<TcpStreamTLS>>, public_service: Arc<Mutex<PublicService>>, client_service: Arc<Mutex<ClientService>>, client_id: String) {
    info!("Tunnel handler started.");
    // sleep for 1 seconds to prevent race condition with healthcheck packet
    sleep(Duration::from_secs(1)).await;
    loop {
        // request from the queue
        let raw_request = public_service.lock().await.dequeue_request(client_id.clone()).await;
        if let Err(message) = raw_request {
            // error!("Error getting pending requests: {}", message);
            // sleep(Duration::from_secs(5)).await;
            // check connection validity
            let _ = match send_health_check_packet(stream.clone()).await {
                Ok(ok) => ok,
                Err(err) => {
                    error!("{}", err);
                    break;
                }
            };

            continue;
        }

        let public_request = raw_request.unwrap();
        println!("processing: {}", public_request);
        
        // send request to client service
        let bytes_req = to_json_vec(&public_request.clone());
        let _ = match stream.lock().await.write_all(&bytes_req).await {
            Ok(ok) => ok,
            Err(err) => {
                format!("{}", err);
                break;
            }
        };

        info!("Request: {} was sent to client: {}.", public_request.id, public_request.client_id);

        // get latest response from stream
        let mut raw_response = Vec::new();
        if let Err(e) = read_bytes_from_mutexed_socket(stream.clone(), &mut raw_response).await {
            error!("{}", e);
            break;
        }

        if raw_response.len() == 0 {
            continue;
        }

        // enqueue Public Response
        let response: PublicResponse = from_json_slice(&raw_response).unwrap();
        let _ = match public_service.lock().await.assign_response(response.clone()).await {
            Ok(ok) => ok,
            Err(err) => {
                break;
            }
        };

        info!("Response received for request: {}.", response.request_id);
    }
    
    // disconnection
    client_service.lock().await.disconnect_client(client_id.clone()).await.unwrap();
    info!("Client Disconnected. client_id: {}", client_id);
}
