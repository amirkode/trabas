use std::sync::Arc;

use futures::io;
use http::{Request, Response, StatusCode, Version};
use cookie::{Cookie, CookieJar};
use log::info;
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf}, net::TcpStream, sync::Mutex};
use tokio_native_tls::TlsStream;

use crate::convert::response_to_bytes;

// these values are standard for this tool
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

// WARNING: this is only use for Trabas: internal Server-Client Connection
pub async fn read_bytes_from_socket_for_internal(stream: &mut TcpStreamTLS, res: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0; 1024];
    let end_window = PACKET_SEPARATOR.as_bytes();
    let end_window_len = end_window.len();
    let break_limit = 100;
    let mut break_cnt = 0;
    let mut prev_len = res.len();
    loop {
        let n = stream.read(&mut buffer).await
            .map_err(|e| format!("Error reading socket: {}",  e))?;

        // TODO: check this again, not really sure if it's useful
        // if n == 0 {
        //     return Err(String::from("Error reading socket: Connection closed"));
        // }

        res.extend_from_slice(&buffer[..n]);
        // we break until the last element is the separator
        // because all request must be transfered in a full form
        // TODO: implement breaker for unexpected connection (?)
        if res.len() >= end_window_len && res[(res.len() - end_window_len)..] == *end_window {
            break;
        }

        // try at most the break_limit for any empty transfer
        let curr_len = res.len();
        if prev_len == curr_len {
            if break_cnt == break_limit {
                info!("Socket reading break limit exceeded");
                break;
            }
            break_cnt += 1;
        }
        
        prev_len = curr_len;
    }

    Ok(())
}

// TODO: do we really need to duplicate the code
pub async fn read_bytes_from_mutexed_socket_for_internal(stream: Arc<Mutex<TcpStreamTLS>>, res: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0; 1024];
    let end_window = PACKET_SEPARATOR.as_bytes();
    let end_window_len = end_window.len();
    let break_limit = 100;
    let mut break_cnt = 0;
    let mut prev_len = res.len();
    loop {
        let n = stream.lock().await.read(&mut buffer).await
            .map_err(|e| format!("Error reading socket: {}",  e))?;

        res.extend_from_slice(&buffer[..n]);
        // we break until the last element is the separator
        // because all request must be transfered in a full form
        // TODO: implement breaker for unexpected connection (?)
        if res.len() >= end_window_len && res[(res.len() - end_window_len)..] == *end_window {
            break;
        }

        // try at most the break_limit for any empty transfer
        let curr_len = res.len();
        if prev_len == curr_len {
            if break_cnt == break_limit {
                info!("Socket reading break limit exceeded");
                break;
            }
            break_cnt += 1;
        }
        
        prev_len = curr_len;
    }

    Ok(())
}

pub async fn read_string_from_socket_for_internal(stream: &mut TcpStreamTLS, res: &mut String) -> Result<(), String> {
    let mut temp = Vec::new();
    read_bytes_from_socket_for_internal(stream, &mut temp).await?;
    let packets = separate_packets(temp);
    if let Some(data) = packets.get(0) {
        *res = String::from_utf8(data.clone()).unwrap();
        return Ok(())
    }

    Err(String::from("Error reading string"))
}

// After serveral tries, turned out the `read_bytes_from_socket` is not reliable for reading http response,
// so, we need customized implementation for it
// TODO: reconsider using standard library or popular library for HTTP response reading (?)
pub async fn read_bytes_from_socket_for_http(stream: &mut TcpStreamTLS, res: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0; 1024];
    let break_limit = 100;
    let mut break_cnt = 0;
    let mut prev_len = res.len();
    // reading headers
    loop {
        let n = stream.read(&mut buffer).await.map_err(|e| format!("Error reading socket: {}", e))?;

        res.extend_from_slice(&buffer[..n]);
        if res.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }

        // try at most the break_limit for any empty transfer
        let curr_len = res.len();
        if prev_len == curr_len {
            if break_cnt == break_limit {
                info!("Socket reading break limit exceeded");
                break;
            }
            break_cnt += 1;
        }
        
        prev_len = curr_len;
    }

    // check headers
    let headers_text = String::from_utf8_lossy(&res);
    let headers_end = match headers_text.find("\r\n\r\n") {
        Some(value) => value + 4, // skip \r\n\r\n
        None => {
            return Ok(());
        }
    };
    
    let content_length: Option<usize> = headers_text
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length:").map(|len| len.trim().parse().ok()))
        .flatten();
    let connection: Option<String> = headers_text
        .lines()
        .find_map(|line| line.strip_prefix("Connection:").map(|len| len.trim().to_lowercase().to_string()));
    let is_chunked = headers_text
        .lines()
        .any(|line| {
            line.to_lowercase().starts_with("transfer-encoding:") && 
            line.to_lowercase().contains("chunked")
        });

    if is_chunked {
        // handle chunked data
        let mut body_start = headers_end;
        let mut decoded_body = Vec::new();
        loop {
            // read hex str size of the current chunk
            let mut chunk_size_str = String::new();
            while body_start < res.len() {
                let byte = res[body_start] as char;
                body_start += 1;
                
                if byte == '\r' {
                    if body_start < res.len() && res[body_start] as char == '\n' {
                        body_start += 1;
                        break;
                    }
                } else {
                    chunk_size_str.push(byte);
                }
            }
            
            // convert the hex size to int
            let chunk_size = usize::from_str_radix(chunk_size_str.trim(), 16)
                .map_err(|e| format!("Invalid chunk size: {}", e))?;
            
            // we've reached the end
            if chunk_size == 0 {
                break;
            }
            
            // read the remaining data in chunk
            while (body_start + chunk_size) > res.len() {
                let n = stream.read(&mut buffer).await
                    .map_err(|e| format!("Error reading socket: {}", e))?;
                if n == 0 {
                    return Err("Error reading socket: Connection closed before completing chunked transfer".to_string());
                }

                res.extend_from_slice(&buffer[..n]);
            }
            
            // extract chunk data
            decoded_body.extend_from_slice(&res[body_start..body_start + chunk_size]);
            body_start += chunk_size;
            
            // perform another reading, if ending separator has not been read
            if (body_start + 2) > res.len() {
                let n = stream.read(&mut buffer).await
                    .map_err(|e| format!("Error reading socket: {}", e))?;
                res.extend_from_slice(&buffer[..n]);
            }

            body_start += 2; // skip \r\n (separator)
        }
        
        // Keep headers and replace body with decoded chunks
        res.truncate(headers_end); // Keep only headers
        res.extend_from_slice(&decoded_body); // Add decoded body
        
    } else if let Some(len) = content_length {
        // handle data with Content-Length
        let target_len = headers_end + len;
        
        break_cnt = 0;
        prev_len = res.len();

        while prev_len < target_len {
            let n = stream.read(&mut buffer).await
                .map_err(|e| format!("Error reading socket: {}", e))?;

            res.extend_from_slice(&buffer[..n]);

            // try at most the break_limit for any empty transfer
            let curr_len = res.len();
            if prev_len == curr_len {
                if break_cnt == break_limit {
                    info!("Socket reading break limit exceeded");
                    break;
                }
                break_cnt += 1;
            }
            
            prev_len = curr_len;
        }
    } else if let Some(connection) = connection {
        if connection == "close" {
            // continue read until end of connection
            loop {
                let n = stream.read(&mut buffer).await.map_err(|e| format!("Error reading socket: {}", e))?;
                if n == 0 {
                    break;
                }
                
                res.extend_from_slice(&buffer[..n]);
            }
        }
    }

    Ok(())
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

// get cookie from request headers
pub fn get_cookie_from_request<T>(req: &Request<T>, cookie_name: &str) -> Option<String> {
    let cookie_header = req.headers().get("cookie")?;
    let cookie_str = cookie_header.to_str().ok()?;

    let mut jar = CookieJar::new();
    for cookie in cookie_str.split("; ") {
        if let Ok(parsed_cookie) = Cookie::parse(cookie.to_string()) {
            jar.add(parsed_cookie);
        }
    }

    jar.get(cookie_name).map(|cookie| cookie.value().to_string())
}
