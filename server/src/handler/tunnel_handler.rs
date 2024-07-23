
use common::convert::{from_json_slice, to_json_vec};
use common::net::{read_bytes_from_mutexed_socket, read_bytes_from_socket, send_health_check_packet};
use common::validate_signature;
use log::{error, info};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use common::data::dto::public_response::PublicResponse;
use common::data::dto::tunnel_client::TunnelClient;
use crate::config;
use crate::service::client_service::ClientService;
use crate::service::public_service::PublicService;

pub async fn register_tunnel_handler(mut stream: TcpStream, client_service: ClientService, public_service: PublicService) -> () {
    info!("Pending connection");
    // register client ID
    let mut raw_response = Vec::new();
    if let Err(e) = read_bytes_from_socket(&mut stream, &mut raw_response).await {
        error!("{}", e);
        return;
    }

    info!("Done reading connection");
    let client: TunnelClient = from_json_slice(&raw_response).unwrap();
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
    let stream_dc1 = Arc::new(Mutex::new(false));
    let stream_dc2 = Arc::clone(&stream_dc1);
    let stream_dc3 = Arc::clone(&stream_dc1);
    let stream1 = Arc::new(Mutex::new(stream));
    let stream2 = Arc::clone(&stream1);
    let client_service1 = Arc::new(Mutex::new(client_service));
    let public_service1 = Arc::new(Mutex::new(public_service));
    let public_service2 = Arc::clone(&public_service1);

    // spawn sender handler
    tokio::spawn(async move {
        tunnel_handler(stream1, public_service1, client_service1, client_id).await;
    });

    // spawn receiver handler
    // tokio::spawn(async move {
    //     receiver_handler(stream_dc2, stream2, public_service2).await;
    // });

    // spawn connection checker
    // tokio::spawn(async move {
    //     connection_checker(stream_dc3, client_service1, client_id).await;
    // });
}

fn validate_connection(signature: String, client_id: String) -> bool {
    let secret = std::env::var(config::CONFIG_KEY_SERVER_SECRET).unwrap_or_default();
    validate_signature!(signature, client_id, secret)
}

async fn connection_checker(stream_dc: Arc<Mutex<bool>>, service: Arc<Mutex<ClientService>>, client_id: String) {
    loop {
        let dc = {
            let guard = stream_dc.lock().await;
            *guard
        };
        if dc {
            service.lock().await.disconnect_client(client_id).await.unwrap();
            break;
        }

        // add break interval for 2 seconds
        sleep(Duration::from_secs(2)).await;
    }
}

async fn tunnel_handler(stream: Arc<Mutex<TcpStream>>, public_service: Arc<Mutex<PublicService>>, client_service: Arc<Mutex<ClientService>>, client_id: String) {
    info!("Tunnel handler started.");
    loop {
        // request from the queue
        let raw_request = public_service.lock().await.dequeue_request().await;
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

async fn receiver_handler(stream_dc: Arc<Mutex<bool>>, stream: Arc<Mutex<TcpStream>>, service: Arc<Mutex<PublicService>>) {
    info!("Receiver handler started.");
    loop {
        // check stream connection
        let dc = {
            let guard = stream_dc.lock().await;
            *guard
        };
        if dc {
            break;
        }

        // get latest response from stream
        let mut raw_response = Vec::new();
        read_bytes_from_mutexed_socket(stream.clone(), &mut raw_response).await;

        if raw_response.len() == 0 {
            continue;
        }

        // enqueue Public Response
        let response: PublicResponse = from_json_slice(&raw_response).unwrap();
        service.lock().await.assign_response(response.clone()).await.unwrap();

        info!("Response received for request: {}.", response.request_id);
    }

    // always update state if the loop exited
    let mut guard = stream_dc.lock().await;
    *guard = true;
}
