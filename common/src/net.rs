use std::{future::Future, pin::Pin, sync::Arc, task::{Context, Poll}};

use futures::io;
use http::{Response, StatusCode, Version};
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf}, net::TcpStream, sync::Mutex};
use tokio_native_tls::TlsStream;

use crate::convert::response_to_bytes;

// TODO: could've been better uniqueness (?)
pub const HEALTH_CHECK_PACKET: &str = "hc_b451f06";
pub const HEALTH_CHECK_PACKET_ACK: &str = "hc_1565b85_ack";

pub const PACKET_SEPARATOR: &str = "$672d20a$";

// make tls as an option
pub struct TcpStreamTLS {
    pub tcp_read: Option<ReadHalf<TcpStream>>,
    pub tcp_write: Option<WriteHalf<TcpStream>>,
    pub tcp_tls_read: Option<ReadHalf<TlsStream<TcpStream>>>,
    pub tcp_tls_write: Option<WriteHalf<TlsStream<TcpStream>>>
}

impl TcpStreamTLS {
    pub fn from_tcp(tcp_read: ReadHalf<TcpStream>, tcp_write: WriteHalf<TcpStream>) -> Self {
        TcpStreamTLS {
            tcp_read: Some(tcp_read),
            tcp_write: Some(tcp_write),
            tcp_tls_read: None,
            tcp_tls_write: None
        }
    }

    pub fn from_tcp_read(tcp: ReadHalf<TcpStream>) -> Self {
        TcpStreamTLS {
            tcp_read: Some(tcp),
            tcp_write: None,
            tcp_tls_read: None,
            tcp_tls_write: None
        }
    }

    pub fn from_tcp_write(tcp: WriteHalf<TcpStream>) -> Self {
        TcpStreamTLS {
            tcp_read: None,
            tcp_write: Some(tcp),
            tcp_tls_read: None,
            tcp_tls_write: None
        }
    }

    pub fn from_tcp_tls(tcp_tls_read: ReadHalf<TlsStream<TcpStream>>, tcp_tls_write: WriteHalf<TlsStream<TcpStream>>) -> Self {
        TcpStreamTLS {
            tcp_read: None,
            tcp_write: None,
            tcp_tls_read: Some(tcp_tls_read),
            tcp_tls_write: Some(tcp_tls_write)
        }
    }

    pub fn from_tcp_tls_read(tcp: ReadHalf<TlsStream<TcpStream>>) -> Self {
        TcpStreamTLS {
            tcp_read: None,
            tcp_write: None,
            tcp_tls_read: Some(tcp),
            tcp_tls_write: None
        }
    }

    pub fn from_tcp_tls_write(tcp: WriteHalf<TlsStream<TcpStream>>) -> Self {
        TcpStreamTLS {
            tcp_read: None,
            tcp_write: None,
            tcp_tls_read: None,
            tcp_tls_write: Some(tcp)
        }
    }

    pub fn use_tls(&self) -> bool {
        self.tcp_tls_read.is_some() || self.tcp_tls_write.is_some()
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.use_tls() {
            self.tcp_tls_read.as_mut().unwrap().read(buf).await
        } else {
            self.tcp_read.as_mut().unwrap().read(buf).await
        }
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.use_tls() {
            self.tcp_tls_write.as_mut().unwrap().write_all(buf).await
        } else {
            self.tcp_write.as_mut().unwrap().write_all(buf).await
        }
    }
}

// IMPORTANT
// we assume all packat exchange between server and client is string serialable

pub fn prepare_packet(mut data: Vec<u8>) -> Vec<u8> {
    let separator: Vec<u8> = Vec::from(PACKET_SEPARATOR.as_bytes());
    data.extend(&separator);
    data
}

pub fn separate_packets(data: Vec<u8>) -> Vec<Vec<u8>> {
    let raw_string = String::from_utf8(data).unwrap();
    let mut res: Vec<Vec<u8>> = Vec::new();
    for packet in raw_string.split(PACKET_SEPARATOR) {
        let trimmed_packet = packet.trim();
        if trimmed_packet == "" || trimmed_packet == HEALTH_CHECK_PACKET_ACK {
            continue;
        }
        
        res.push(Vec::from(trimmed_packet.as_bytes()));
    }

    res
}

pub async fn ack_health_check_packet(stream: Arc<Mutex<TcpStreamTLS>>, data: Vec<u8>) -> bool { 
    let str_data = String::from_utf8(data).unwrap();
    if str_data != HEALTH_CHECK_PACKET {
        return false;
    }

    stream.lock().await.write_all(String::from(HEALTH_CHECK_PACKET_ACK).as_bytes()).await.unwrap_or_default();
    true
}

pub async fn send_health_check_packet(stream: Arc<Mutex<TcpStreamTLS>>) -> Result<(), String> {
    stream.lock().await.write_all(String::from(HEALTH_CHECK_PACKET).as_bytes()).await
        .map_err(|e| format!("Error sending health check packet: {}",  e))?;
    let mut ack = String::default();
    read_string_from_mutexed_socket(stream, &mut ack).await;
    if ack != HEALTH_CHECK_PACKET_ACK {
        return Err(String::from("Health check failed"));
    }
    Ok(())
}

// TODO: reading readiness using poll ready state might be a good option (?)
// but it does not seem to be reliable for checking e2e connection
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

pub async fn read_bytes_from_socket(stream: &mut TcpStreamTLS, res: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0; 1024];
    loop {
        let n = stream.read(&mut buffer).await
            .map_err(|e| format!("Error reading socket: {}",  e))?;

        // TODO: check this again, not really sure if it's useful
        // if n == 0 {
        //     return Err(String::from("Error reading socket: Connection closed"));
        // }

        res.extend_from_slice(&buffer[..n]);
        if res.windows(4).any(|window| window == b"\r\n\r\n") || n < buffer.len() {
            break;
        }
    }

    Ok(())
}

// TODO: do we readlly need to duplicate the code
pub async fn read_bytes_from_mutexed_socket(stream: Arc<Mutex<TcpStreamTLS>>, res: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0; 1024];
    loop {
        let n = stream.lock().await.read(&mut buffer).await
            .map_err(|e| format!("Error reading socket: {}",  e))?;

        res.extend_from_slice(&buffer[..n]);
        if res.windows(4).any(|window| window == b"\r\n\r\n") || n < buffer.len() {
            break;
        }
    }

    Ok(())
}

pub async fn read_string_from_socket(stream: &mut TcpStreamTLS, res: &mut String) -> Result<(), String> {
    let mut temp = Vec::new();
    read_bytes_from_socket(stream, &mut temp).await?;
    *res = String::from_utf8(temp).unwrap();
    Ok(())
}

pub async fn read_string_from_mutexed_socket(stream: Arc<Mutex<TcpStreamTLS>>, res: &mut String) {
    let mut temp = Vec::new();
    read_bytes_from_mutexed_socket(stream, &mut temp).await.unwrap();
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

// returns string response for http request
pub fn http_string_response_as_bytes(response: String, status: StatusCode) -> Result<Vec<u8>, String> {
    let res_bytes = response.as_bytes();
    let content_length = res_bytes.len();
    let res = Response::builder()
        .version(Version::HTTP_11)
        .status(status)
        .header("Content-Type", "application/json")
        .header("Content-Length", content_length.to_string())
        .body(Vec::from(res_bytes))
        .map_err(|e| format!("Error building json response: {}", e))?;

    Ok(response_to_bytes(&res))
}
