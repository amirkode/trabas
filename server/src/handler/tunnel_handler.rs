
use log::error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;
use common::data::dto::public_response::PublicResponse;
use crate::service::client_service::ClientService;
use crate::service::public_service::PublicService;

pub async fn register_tunnel_handler(mut stream: TcpStream, client_service: ClientService, public_service: PublicService) {
    // register client ID
    let mut client_id = String::new();
    stream.read_to_string(&mut client_id).await.unwrap();
    let _ = client_service.register_client(client_id.clone());

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
        stream.lock().await.write_all(&raw_request.unwrap()).await.unwrap();
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
        let response: PublicResponse = serde_json::from_slice(&raw_response).unwrap();
        service.lock().await.assign_response(response).await.unwrap();
    }

    // always update state if the loop exited
    let mut guard = stream_dc.lock().await;
    *guard = true;
}
