use common::net::{read_bytes_from_socket_for_http, TcpStreamTLS};
use tokio::net::TcpStream;
use async_trait::async_trait;

// TODO: couldn't think of a better name, might change it in the future.
#[async_trait]
pub trait UnderlyingRepo: Send + Sync {
    async fn forward(&self, request: Vec<u8>, host: String) -> Result<Vec<u8>, String>;
}

pub struct UnderlyingRepoImpl { }

impl UnderlyingRepoImpl {
    pub fn new() -> Self {
        UnderlyingRepoImpl {  }
    }
}

#[async_trait]
impl UnderlyingRepo for UnderlyingRepoImpl {
    async fn forward(&self, request: Vec<u8>, host: String) -> Result<Vec<u8>, String> {
        //info!("Forwarding request: {} to host: {}", String::from_utf8(request.clone()).unwrap(), host.clone());
        let stream = TcpStream::connect(host.as_str()).await.unwrap();
        let (read_stream, write_stream) = tokio::io::split(stream);
        let mut stream = TcpStreamTLS::from_tcp(read_stream, write_stream);
        
        // forward request
        stream.write_all(&request).await.unwrap();
        
        // read response
        let mut res = Vec::new();
        read_bytes_from_socket_for_http(&mut stream, &mut res).await?;

        // let res_str = match String::from_utf8(res.clone()) {
        //     Ok(value) => value,
        //     Err(err) => format!("err: {}", err)
        // };
        
        // info!("underlying service response:\n{}", res_str);

        Ok(res)
    }
}