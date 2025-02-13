#[cfg(test)]
mod tests {
    use common::*;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::io::AsyncWriteExt;
    use http::{Request, StatusCode};

    #[tokio::test]
    async fn test_prepare_packet() {
        let data = Vec::from("test data".as_bytes());
        let packet = net::prepare_packet(data.clone());
        let expected_packet = Vec::from(format!("{}{}", String::from_utf8(data).unwrap(), net::PACKET_SEPARATOR).as_bytes());
        assert_eq!(packet, expected_packet);
    }

    #[tokio::test]
    async fn test_separate_packets() {
        let packet1 = Vec::from("packet1".as_bytes());
        let packet2 = Vec::from("packet2".as_bytes());
        let mut data = net::prepare_packet(packet1.clone());
        data.extend(net::prepare_packet(packet2.clone()));
        data.extend(Vec::from(net::HEALTH_CHECK_PACKET_ACK.as_bytes()));
        data.extend(net::prepare_packet(Vec::new()));

        let packets = net::separate_packets(data);
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[0], packet1);
        assert_eq!(packets[1], packet2);
    }

    #[tokio::test]
    async fn test_tcp_stream_tls_read_write() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, write) = tokio::io::split(socket);
            let mut stream = net::TcpStreamTLS::from_tcp(read, write);

            stream.write_all("hello".as_bytes()).await.unwrap();

            let mut buf = [0u8; 5];
            stream.read(&mut buf).await.unwrap();
            assert_eq!(&buf, b"hello");
        });

        let socket = TcpStream::connect(addr).await.unwrap();
        let (read, write) = tokio::io::split(socket);
        let mut stream = net::TcpStreamTLS::from_tcp(read, write);

        stream.write_all("hello".as_bytes()).await.unwrap();

        let mut buf = [0u8; 5];
        stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello");
    }

    #[tokio::test]
    async fn test_http_json_response_as_bytes() {
        let response = net::HttpResponse::new(true, "test message".to_string());
        let status = StatusCode::OK;
        let bytes = net::http_json_response_as_bytes(response, status).unwrap();
        let body = String::from_utf8(bytes).unwrap();
        assert!(body.contains("test message"));
    }

    #[tokio::test]
    async fn test_http_string_response_as_bytes() {
        let response = "test message".to_string();
        let status = StatusCode::OK;
        let bytes = net::http_string_response_as_bytes(response, status).unwrap();
        let body = String::from_utf8(bytes).unwrap();
        assert!(body.contains("test message"));
    }

    #[tokio::test]
    async fn test_get_cookie_from_request() {
        let req = Request::builder()
            .header("Cookie", "name=value; other=other_value")
            .body(())
            .unwrap();

        let cookie = net::get_cookie_from_request(&req, "name").unwrap();
        assert_eq!(cookie, "value");

        let cookie = net::get_cookie_from_request(&req, "nonexistent");
        assert_eq!(cookie, None);
    }

    #[tokio::test]
    async fn test_http_reader() {
        use tokio::io::AsyncRead;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct MockTcpStream {
            data: Vec<u8>,
            position: usize,
        }

        impl MockTcpStream {
            fn new(data: Vec<u8>) -> Self {
                MockTcpStream { data, position: 0 }
            }
        }

        impl AsyncRead for MockTcpStream {
            fn poll_read(
                mut self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &mut tokio::io::ReadBuf<'_>,
            ) -> Poll<std::io::Result<()>> {
                let remaining = self.data.len() - self.position;
                if remaining == 0 {
                    return Poll::Ready(Ok(()));
                }

                let len = std::cmp::min(buf.remaining(), remaining);
                buf.put_slice(&self.data[self.position..self.position + len]);
                self.position += len;
                Poll::Ready(Ok(()))
            }
        }

        impl tokio::io::AsyncWrite for MockTcpStream {
            fn poll_write(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<Result<usize, std::io::Error>> {
                Poll::Ready(Ok(buf.len()))
            }

            fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
                Poll::Ready(Ok(()))
            }

            fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
                Poll::Ready(Ok(()))
            }
        }

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let (read, write) = tokio::io::split(socket);
            let mut stream = net::TcpStreamTLS::from_tcp(read, write);
            let mut reader = net::HttpReader::from_tcp_stream(&mut stream);
            let mut res = Vec::new();
            reader.read(&mut res, false).await.unwrap();
        });

        let stream = TcpStream::connect(addr).await.unwrap();
        let (_, mut write) = tokio::io::split(stream);
        write.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, world!").await.unwrap();
    }
}