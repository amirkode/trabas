use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};
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
        let mut stream = TcpStream::connect(host.as_str()).await.unwrap();
        // forward request
        stream.write_all(&request).await.unwrap();
        
        // read response
        let mut res = Vec::new();
        stream.read_to_end(&mut res).await.unwrap();
        stream.shutdown().await.unwrap();

        Ok(res)
    }
}