// TODO: implement this

use http::{Response, Version};

use common::{
    convert::{from_json_slice, response_to_bytes, to_json_vec}, 
    data::dto::{public_request::PublicRequest, public_response::PublicResponse, tunnel_client::TunnelClient}, 
    security::sign_value
};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
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
    stream.read_to_string(&mut ok).await.unwrap();
    if ok != "ok" {
        error!("Error connecting to server service: {}", ok);
        return;
    }

    info!("Connected to server service.");

    // spawn tunnel handler
    tokio::spawn(async move {
        tunnel_handler(stream, underlying_host, service).await;
    });
}

fn get_tunnel_client() -> TunnelClient {
    let client_id = std::env::var(config::CONFIG_KEY_CLIENT_ID)
        .expect(format!("{} env has not been set", config::CONFIG_KEY_CLIENT_ID).as_str());
    let signing_key = std::env::var(config::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY)
        .expect(format!("{} env has not been set", config::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY).as_str());
    let signature = sign_value(client_id.clone(), signing_key);
    TunnelClient::new(client_id, signature)
}

pub async fn tunnel_handler(mut stream: TcpStream, underlying_host: String, service: UnderlyingService) {
    info!("Tunnel handler started.");
    loop {
        // get incoming request server service to forward
        let mut request = Vec::new();
        stream.read_to_end(&mut request).await.unwrap();

        let public_request: PublicRequest = from_json_slice(&request).unwrap();
        info!("Incoming request: {} received.", public_request.id);

        // forward response to underlying service
        let res = service.foward_request(request, underlying_host.clone()).await;
        if res.is_err() {
            // response error to server
            let response = Response::builder()
            .version(Version::HTTP_11)
            .status(400)
            .header("Content-Type", "text/plain")
            .body(String::from("Request cannot be processed"))
            .unwrap();

            stream.write_all(&response_to_bytes(&response)).await.unwrap();

            continue;
        }
        
        let res = res.unwrap();
        let public_response = PublicResponse::new(public_request.id.clone(), res.clone());

        // foward response from underlying service to server service
        let bytes_res = to_json_vec(&public_response);
        stream.write_all(&bytes_res).await.unwrap();

        info!("Incoming request: {} processed.", public_request.id);
    }
}