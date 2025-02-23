use std::{sync::Arc, time::Duration};

use http::StatusCode;

use common::{
    convert::{from_json_slice, to_json_vec}, 
    data::dto::{public_request::PublicRequest, public_response::PublicResponse, tunnel_ack::TunnelAck, tunnel_client::TunnelClient}, 
    net::{
        http_json_response_as_bytes, 
        prepare_packet, 
        read_bytes_from_mutexed_socket_for_internal, 
        read_bytes_from_socket_for_internal, 
        separate_packets, 
        HttpResponse, 
        TcpStreamTLS, 
        HEALTH_CHECK_PACKET_ACK
    }, 
    security::sign_value
};
use tokio::{net::TcpStream, sync::{Mutex, mpsc, mpsc::{Sender, Receiver}}, time::{sleep, Instant}};
use tokio_native_tls::{native_tls, TlsConnector};

use common::{_error, _info};
use common::config::keys as config_keys;
use crate::{config::get_ca_certificate, service::underlying_service::UnderlyingService};

pub async fn register_handler(underlying_host: String, service: UnderlyingService, use_tls: bool) -> () {
    // TODO: add initial connection validation for underlying service
    let server_host = std::env::var(config_keys::CONFIG_KEY_CLIENT_SERVER_HOST).unwrap_or_default();
    let server_port = std::env::var(config_keys::CONFIG_KEY_CLIENT_SERVER_PORT).unwrap_or_default();
    let server_address = format!("{}:{}", server_host, server_port);
    let debug = std::env::var(config_keys::CONFIG_KEY_GLOBAL_DEBUG).unwrap_or_default() == "true";
    // is one thousands a good enough limit (?)
    let mut max_tries = 1000;
    while max_tries > 0 {
        if max_tries < 1000 {
            // add break interval for 5 seconds
            _info!("Break for 5 seconds for the next attempt.");
            sleep(Duration::from_secs(5)).await;
        }

        max_tries -= 1;

        _info!("Attempting to connect to server service{}...", if debug { format!(" at [{}]", server_address.clone()) } else { "".to_string() }); 
        let tcp_stream = match TcpStream::connect(server_address.clone()).await {
            Ok(ok) => ok,
            Err(e) => {
                _error!("Error connecting to server service at {}: {}", server_address.clone(), e);
                continue;
            }
        };

        _info!("Initial connection established."); 

        let (mut read_stream, mut write_stream) = if use_tls {
            _info!("TLS option enabled. Binding to TLS...");
            let cert = get_ca_certificate().unwrap();
            let connector = native_tls::TlsConnector::builder()
                .add_root_certificate(cert)
                .build()
                .unwrap();
            let connector = TlsConnector::from(connector);
            let tls_stream = connector.connect(server_host.as_str(), tcp_stream).await.unwrap();
            let (read_stream, write_stream) = tokio::io::split(tls_stream);
            _info!("TLS binding successful.");
            (TcpStreamTLS::from_tcp_tls_read(read_stream), TcpStreamTLS::from_tcp_tls_write(write_stream))
        } else { 
            let (read_stream, write_stream) = tokio::io::split(tcp_stream);
            (TcpStreamTLS::from_tcp_read(read_stream), TcpStreamTLS::from_tcp_write(write_stream))
         };
        // send connection request to server service
        let tunnel_client = get_tunnel_client();
        let packet = prepare_packet(to_json_vec(&tunnel_client));

        _info!("Connecting to server service for authentication and registration...");
        
        if let Err(e) = write_stream.write_all(&packet).await {
            _error!("Failed to send authentication packet: {}", e);
            continue;
        }
        
        let mut server_response = Vec::new();
        if let Err(e) = read_bytes_from_socket_for_internal(&mut read_stream, &mut server_response).await {
            _error!("Failed to read server service response: {}", e);
            continue;
        }
        
        let packets = separate_packets(server_response);
        let server_response = match packets.get(0) {
            Some(data) => data,
            None => {
                _error!("Handshake failed: Empty response from server service.");
                return;
            }
        };
        
        let ack: TunnelAck = match from_json_slice(&server_response) {
            Some(value) => value,
            None => {
                _error!("Handshake failed: Invalid JSON response from server service.");
                return;    
            }
        };
        
        if !ack.success {
            _error!("Server service rejected connection: {}", ack.message);
            continue;
        }

        _info!("Successfully authenticated and registered with the server service.");
        _info!(raw: "Available Public Endpoints:");
        for endpoint in ack.public_endpoints {
            _info!("* `{}`", endpoint);
        }
        
        // create channel for request queue
        let (tx, rx) = mpsc::channel::<PublicResponse>(5);

        // convert to mutex
        let tx_mutex = Arc::new(Mutex::new(tx));
        let rx_mutex = Arc::new(Mutex::new(rx));
        let read_stream_mutex = Arc::new(Mutex::new(read_stream));
        let write_stream_mutex = Arc::new(Mutex::new(write_stream));
        
        // share handler stop state between sender and reciever
        let handler_stopped1 = Arc::new(Mutex::new(false));
        let handler_stopped2 = handler_stopped1.clone();

        // spawn handlers
        let cloned_underlying_host = underlying_host.clone();
        let cloned_service = service.clone();
        let receiver_handler = tokio::spawn(async move {
            tunnel_receiver_handler(handler_stopped1, read_stream_mutex, tx_mutex, cloned_underlying_host, cloned_service).await;
        });
        let sender_handler = tokio::spawn(async move {
            tunnel_sender_handler(handler_stopped2, write_stream_mutex, rx_mutex).await;
        });

        // wait until released
        receiver_handler.await.unwrap_or_default();
        sender_handler.await.unwrap_or_default();
    }

    _info!("Max server binding retries exceeded.");
}

fn get_tunnel_client() -> TunnelClient {
    let client_id = std::env::var(config_keys::CONFIG_KEY_CLIENT_ID)
        .expect(format!("{} env has not been set", config_keys::CONFIG_KEY_CLIENT_ID).as_str());
    let signing_key = std::env::var(config_keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY)
        .expect(format!("{} env has not been set", config_keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY).as_str());
    let signature = sign_value(client_id.clone(), signing_key);
    TunnelClient::new(client_id, signature)
}

pub async fn tunnel_receiver_handler(
    handler_stopped: Arc<Mutex<bool>>,
    stream: Arc<Mutex<TcpStreamTLS>>, 
    tx: Arc<Mutex<Sender<PublicResponse>>>, 
    underlying_host: String, 
    service: UnderlyingService
) {
    _info!("Tunnel receiver handler started.");
    while !(*handler_stopped.lock().await) {
        // get incoming request server service to forward
        let mut request = Vec::new();
        if let Err(e) = read_bytes_from_mutexed_socket_for_internal(stream.clone(), &mut request).await {
            _error!("{}", e);
            break;
        }

        if request.len() == 0 {
            // no request found yet
            continue;
        }

        let packets = separate_packets(request);
        for packet in packets {
            let public_request: PublicRequest = from_json_slice(&packet).unwrap(); // assuming correct format
            let start_request = Instant::now();
            _info!("Incoming request: {} received, forwarding to underlying service...", public_request.id);
            
            // dispatch request to underlying service
            let cloned_underlying_host = underlying_host.clone();
            let cloned_service = service.clone();
            let cloned_tx = tx.clone();
            tokio::spawn(async move {
                let public_response = match cloned_service.foward_request(public_request.data, cloned_underlying_host).await {
                    Ok(res) => {
                        PublicResponse::new(public_request.id.clone(), res.clone())
                    },
                    Err(err) => {
                        _error!("Request [{}] cannot be processed: {}", public_request.id.clone(), err);
                        let msg = String::from("Request cannot be processed");
                        let res = http_json_response_as_bytes(
                            HttpResponse::new(false, msg), StatusCode::from_u16(400).unwrap()).unwrap();
                        PublicResponse::new(public_request.id.clone(), res.clone())
                    }
                };
        
                if let Ok(_) = cloned_tx.lock().await.send(public_response).await {
                    _info!("Response for request {} received in {} seconds and was enqueued to forward back.", public_request.id, start_request.elapsed().as_secs());
                }
            });
        }
    }

    // update handler stop state
    (*handler_stopped.lock().await) = true;

    _info!("Tunnel receiver handler stopped.");
}

pub async fn tunnel_sender_handler(
    handler_stopped: Arc<Mutex<bool>>,
    stream: Arc<Mutex<TcpStreamTLS>>,
    rx: Arc<Mutex<Receiver<PublicResponse>>>
) {
    _info!("Tunnel sender handler started.");
    let mut skip = 0;
    while !(*handler_stopped.lock().await) {
        // get ready public responses from the queue
        if let Some(public_response) = rx.lock().await.recv().await {
            _info!("Response for request: {} is available.", public_response.request_id);
            // foward response from underlying service to server service
            let bytes_res = prepare_packet(to_json_vec(&public_response));
            if let Err(e) = stream.lock().await.write_all(&bytes_res).await {
                _error!("{}", e);
                break;
            }

            _info!("Request: {} processed.", public_response.request_id);
        } else {
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

    // update handler stop state
    (*handler_stopped.lock().await) = true;

    _info!("Tunnel sender handler stopped.");
}
