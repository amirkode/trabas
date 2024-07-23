// TODO: implement this

use std::sync::Arc;

use http::StatusCode;

use common::{
    convert::{from_json_slice, to_json_vec}, 
    data::dto::{public_request::PublicRequest, public_response::PublicResponse, tunnel_client::TunnelClient}, 
    net::{ack_health_check_packet, http_json_response_as_bytes, read_bytes_from_mutexed_socket, read_string_from_socket, HttpResponse}, 
    security::sign_value
};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex, time::Instant};
use log::{error, info};

use crate::{config, service::underlying_service::UnderlyingService};

pub async fn register_handler(mut stream: TcpStream, underlying_host: String, service: UnderlyingService) -> () {
    // send connection request to server service
    let tunnel_client = get_tunnel_client();
    let bytes_req = to_json_vec(&tunnel_client);

    info!("Connecting to server service...");
    stream.write_all(&bytes_req).await.unwrap();
    // check if the server handshake was successful
    let mut ok: String = Default::default();
    read_string_from_socket(&mut stream, &mut ok).await;
    if ok != "ok" {
        error!("Error connecting to server service: {}", ok);
        return;
    }

    info!("Connected to server service.");

    let stream_mutex = Arc::new(Mutex::new(stream));
    // start tunnel handler
    tunnel_handler(stream_mutex, underlying_host, service).await;
}

fn get_tunnel_client() -> TunnelClient {
    let client_id = std::env::var(config::CONFIG_KEY_CLIENT_ID)
        .expect(format!("{} env has not been set", config::CONFIG_KEY_CLIENT_ID).as_str());
    let signing_key = std::env::var(config::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY)
        .expect(format!("{} env has not been set", config::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY).as_str());
    let signature = sign_value(client_id.clone(), signing_key);
    TunnelClient::new(client_id, signature)
}

pub async fn tunnel_handler(stream: Arc<Mutex<TcpStream>>, underlying_host: String, service: UnderlyingService) {
    info!("Tunnel handler started.");
    loop {
        // get incoming request server service to forward
        let mut request = Vec::new();
        read_bytes_from_mutexed_socket(stream.clone(), &mut request).await;
        if request.len() == 0 {
            // no request found yet
            continue;
        }

        if ack_health_check_packet(stream.clone(), request.clone()).await {
            // skip if the packet
            continue;
        }

        let public_request: PublicRequest = from_json_slice(&request).unwrap();
        let start_request = Instant::now();
        info!("Incoming request: {} received, forwarding to underlying service...", public_request.id);
        // forward response to underlying service
        let res = service.foward_request(public_request.data, underlying_host.clone()).await;
        if res.is_err() {
            // response error to server
            let msg = String::from("Request cannot be processed");
            let response = match http_json_response_as_bytes(
                HttpResponse::new(false, msg), StatusCode::from_u16(400).unwrap()) {
                    Ok(value) => value,
                    Err(err) => {
                        info!("error: {}", err);
                        continue;
                    } 
                };

            stream.lock().await.write_all(&response).await.unwrap();

            continue;
        }

        info!("Response for request {} received in {} seconds.", public_request.id,start_request.elapsed().as_secs());
        
        let res = res.unwrap();
        let public_response = PublicResponse::new(public_request.id.clone(), res.clone());

        // foward response from underlying service to server service
        let bytes_res = to_json_vec(&public_response);
        stream.lock().await.write_all(&bytes_res).await.unwrap();

        info!("Incoming request: {} processed.", public_request.id);
    }
}
