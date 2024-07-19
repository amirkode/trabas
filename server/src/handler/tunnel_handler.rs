
use common::{from_json_slice, to_json_vec, validate_signature};
use log::error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use common::data::dto::public_response::PublicResponse;
use common::data::dto::tunnel_client::TunnelClient;
use crate::service::client_service::ClientService;
use crate::service::public_service::PublicService;

pub async fn register_tunnel_handler(mut stream: TcpStream, client_service: ClientService, public_service: PublicService) -> () {
    // register client ID
    let mut raw_response = Vec::new();
    stream.read_to_end(&mut raw_response).await.unwrap();
    let client = from_json_slice!(&raw_response, TunnelClient).unwrap();
    let client_id = client.id.clone();
    // validate connection before registering client
    if !validate_connection(client.signature.clone(), client.id.clone()) {
        let err_msg = format!("Client Registration Denied. client_id: {}, signature: {}", client_id, client.signature);
        stream.write_all(err_msg.as_bytes()).await.unwrap();
        return;
    }
    client_service.register_client(client).unwrap();

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
        sender_handler(stream_dc1, stream1, public_service1).await;
    });

    // spawn receiver handler
    tokio::spawn(async move {
        receiver_handler(stream_dc2, stream2, public_service2).await;
    });

    // spawn connection checker
    tokio::spawn(async move {
        connection_checker(stream_dc3, client_service1, client_id).await;
    });
}

fn validate_connection(signature: String, client_id: String) -> bool {
    let secret = std::env::var("SERVER_SECRET").unwrap_or_default();
    validate_signature!(signature, client_id, secret)
}

async fn connection_checker(stream_dc: Arc<Mutex<bool>>, service: Arc<Mutex<ClientService>>, client_id: String) {
    loop {
        let dc = {
            let guard = stream_dc.lock().await;
            *guard
        };
        if dc {
            service.lock().await.disconnect_client(client_id).unwrap();
            break;
        }

        // add break interval for 2 seconds
        sleep(Duration::from_secs(2)).await;
    }
}

async fn sender_handler(stream_dc: Arc<Mutex<bool>>, stream: Arc<Mutex<TcpStream>>, service: Arc<Mutex<PublicService>>) {
    loop {
        // check stream connection
        let dc = {
            let guard = stream_dc.lock().await;
            *guard
        };
        if dc {
            break;
        }

        // request from the queue
        let raw_request = service.lock().await.dequeue_request().await;
        if let Err(message) = raw_request {
            error!("Error getting pending requests: {}", message);
            continue;
        }
        
        // send request to client service
        let bytes_req = to_json_vec!(raw_request.unwrap());
        stream.lock().await.write_all(&bytes_req).await.unwrap();
    }

    // always update state if the loop exited
    let mut guard = stream_dc.lock().await;
    *guard = true;
}

async fn receiver_handler(stream_dc: Arc<Mutex<bool>>, stream: Arc<Mutex<TcpStream>>, service: Arc<Mutex<PublicService>>) {
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
        stream.lock().await.read_to_end(&mut raw_response).await.unwrap();

        // enqueue Public Response
        let response = from_json_slice!(&raw_response, PublicResponse).unwrap();
        service.lock().await.assign_response(response).await.unwrap();
    }

    // always update state if the loop exited
    let mut guard = stream_dc.lock().await;
    *guard = true;
}
