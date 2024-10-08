// TODO: implement this

use std::{sync::Arc, time::Duration};

use http::StatusCode;

use common::{
    convert::{from_json_slice, to_json_vec}, 
    data::dto::{public_request::PublicRequest, public_response::PublicResponse, tunnel_client::TunnelClient}, 
    net::{ack_health_check_packet, http_json_response_as_bytes, read_bytes_from_mutexed_socket, read_string_from_socket, HttpResponse, TcpStreamTLS}, 
    security::sign_value
};
use tokio::{net::TcpStream, sync::Mutex, time::{sleep, Instant}};
use log::{error, info};
use tokio_native_tls::{native_tls, TlsConnector};

use crate::{config::{self, get_ca_certificate}, service::underlying_service::UnderlyingService};

pub async fn register_handler(underlying_host: String, service: UnderlyingService, use_tls: bool) -> () {
    // TODO: add initial connection validation for underlying service
    let server_host = std::env::var(config::CONFIG_KEY_CLIENT_SERVER_HOST).unwrap_or_default();
    let server_port = std::env::var(config::CONFIG_KEY_CLIENT_SERVER_PORT).unwrap_or_default();
    let server_address = format!("{}:{}", server_host, server_port);
    // is one thousands a good enough limit (?)
    let mut max_tries = 1000;
    while max_tries > 0 {
        if max_tries < 1000 {
            // add break interval for 5 seconds
            info!("Break for 5 second for the next attempt");
            sleep(Duration::from_secs(5)).await;
        }

        max_tries -= 1;

        info!("Binding to server service: {}", server_address.clone()); 
        let tcp_stream = match TcpStream::connect(server_address.clone()).await {
            Ok(ok) => ok,
            Err(e) => {
                error!("Error connecting to {}: {}", server_host, e);
                continue;
            }
        };
        let mut stream = if use_tls {
            let cert = get_ca_certificate().unwrap();
            let connector = native_tls::TlsConnector::builder()
                .add_root_certificate(cert)
                .build()
                .unwrap();
            let connector = TlsConnector::from(connector);
            let tls_stream = connector.connect(server_host.as_str(), tcp_stream).await.unwrap();
            info!("TLS Bound -> address: {}", server_address.clone());
            TcpStreamTLS {
                tcp: None,
                tls: Some(tls_stream)
            }
        } else { 
            TcpStreamTLS {
                tcp: Some(tcp_stream),
                tls: None
            }
         };
        // send connection request to server service
        let tunnel_client = get_tunnel_client();
        let bytes_req = to_json_vec(&tunnel_client);

        info!("Connecting to server service...");
        if let Err(e) = stream.write_all(&bytes_req).await {
            error!("Error connecting to server service: {}", e);
            continue;
        }

        // check if the server handshake was successful
        let mut ok: String = Default::default();
        if let Err(e) = read_string_from_socket(&mut stream, &mut ok).await {
            error!("Error connecting to server service: {}", e);
            continue;
        }
        if ok != "ok" {
            error!("Error connecting to server service: {}", ok);
            continue;
        }

        info!("Connected to server service.");

        let stream_mutex = Arc::new(Mutex::new(stream));
        // start tunnel handler
        tunnel_handler(stream_mutex, underlying_host.clone(), service.clone()).await;
    }
}

fn get_tunnel_client() -> TunnelClient {
    let client_id = std::env::var(config::CONFIG_KEY_CLIENT_ID)
        .expect(format!("{} env has not been set", config::CONFIG_KEY_CLIENT_ID).as_str());
    let signing_key = std::env::var(config::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY)
        .expect(format!("{} env has not been set", config::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY).as_str());
    let signature = sign_value(client_id.clone(), signing_key);
    TunnelClient::new(client_id, signature)
}

pub async fn tunnel_handler(stream: Arc<Mutex<TcpStreamTLS>>, underlying_host: String, service: UnderlyingService) {
    info!("Tunnel handler started.");
    loop {
        // get incoming request server service to forward
        let mut request = Vec::new();
        if let Err(e) = read_bytes_from_mutexed_socket(stream.clone(), &mut request).await {
            error!("{}", e);
            break;
        }

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
        if let Err(e) = stream.lock().await.write_all(&bytes_res).await {
            error!("{}", e);
            break;
        }

        info!("Incoming request: {} processed.", public_request.id);
    }
}
