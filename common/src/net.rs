use std::{future::Future, pin::Pin, sync::Arc, task::{Context, Poll}, time::Duration};

use http::{Response, StatusCode, Version};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, sync::Mutex};

use crate::convert::response_to_bytes;

// TODO: could've been better uniqueness (?)
const HEALTH_CHECK_PACKET: &str = "hc_packet";
const HEALTH_CHECK_PACKET_ACK: &str = "hc_packet_ack";

pub async fn ack_health_check_packet(stream: Arc<Mutex<TcpStream>>, data: Vec<u8>) -> bool { 
    let str_data = String::from_utf8(data).unwrap();
    if str_data != HEALTH_CHECK_PACKET {
        return false;
    }

    stream.lock().await.write_all(String::from(HEALTH_CHECK_PACKET_ACK).as_bytes()).await.unwrap_or_default();
    true
}

pub async fn send_health_check_packet(stream: Arc<Mutex<TcpStream>>) -> Result<(), String> {
    stream.lock().await.write_all(String::from(HEALTH_CHECK_PACKET).as_bytes()).await
        .map_err(|e| format!("Error sending health check packet: {}",  e))?;
    let mut ack = String::default();
    read_string_from_mutexed_socket(stream, &mut ack).await;
    if ack != HEALTH_CHECK_PACKET_ACK {
        return Err(String::from("Health check failed"));
    }
    Ok(())
}

async fn is_socket_readable(stream: &mut TcpStream) -> bool {
    struct ReadReady<'a>(&'a mut TcpStream);

    impl<'a> Future for ReadReady<'a> {
        type Output = std::io::Result<()>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.0.poll_read_ready(cx)
        }
    }

    ReadReady(stream).await.is_ok()
}

pub async fn read_bytes_from_socket(stream: &mut TcpStream, res: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0; 1024];
    loop {
        let n = stream.read(&mut buffer).await
            .map_err(|e| format!("Error reading socket: {}",  e))?;

        if n == 0 {
            return Err(String::from("Error reading socket: Connection closed"));
        }

        res.extend_from_slice(&buffer[..n]);
        if res.windows(4).any(|window| window == b"\r\n\r\n") || n < buffer.len() {
            break;
        }
    }

    Ok(())
}

// TODO: do we readlly need to duplicate the code
pub async fn read_bytes_from_mutexed_socket(stream: Arc<Mutex<TcpStream>>, res: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0; 1024];
    loop {
        let n = stream.lock().await.read(&mut buffer).await
            .map_err(|e| format!("Error reading socket: {}",  e))?;
        if n == 0 {
            return Err(String::from("Error reading socket: Connection closed"));
        }

        res.extend_from_slice(&buffer[..n]);
        if res.windows(4).any(|window| window == b"\r\n\r\n") || n < buffer.len() {
            break;
        }
    }

    Ok(())
}

pub async fn read_string_from_socket(stream: &mut TcpStream, res: &mut String) -> Result<(), String> {
    let mut temp = Vec::new();
    read_bytes_from_socket(stream, &mut temp).await?;
    *res = String::from_utf8(temp).unwrap();
    Ok(())
}

pub async fn read_string_from_mutexed_socket(stream: Arc<Mutex<TcpStream>>, res: &mut String) {
    let mut temp = Vec::new();
    read_bytes_from_mutexed_socket(stream, &mut temp).await;
    *res = String::from_utf8(temp).unwrap();
}

// standard http response for project-wide
#[derive(Serialize, Deserialize, Clone)]
pub struct HttpResponse {
    success: bool,
    message: String
}

impl HttpResponse {
    pub fn new(success: bool, message: String) -> Self {
        HttpResponse { success, message }
    }
}

// returns json response for http request
pub fn http_json_response_as_bytes(response: HttpResponse, status: StatusCode) -> Result<Vec<u8>, String> {
    let json = serde_json::to_string(&response)
        .map_err(|e| format!("Error parsing json response: {}", e))?;
    let json_bytes = json.as_bytes();
    let content_length = json_bytes.len();
    let res = Response::builder()
        .version(Version::HTTP_11)
        .status(status)
        .header("Content-Type", "application/json")
        .header("Content-Length", content_length.to_string())
        .body(Vec::from(json_bytes))
        .map_err(|e| format!("Error building json response: {}", e))?;

    Ok(response_to_bytes(&res))
}
