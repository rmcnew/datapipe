use crate::datapipe_types::InputReader;
/// "Listen for connection, then receive" Reader for TCP
use bytes::Bytes;
use core::net::SocketAddr;
use log::{error, info};
use std::io::{Error, ErrorKind};
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

const TCP_LISTEN_READ_BUFFER_SIZE: usize = 2048;

#[derive(Debug)]
pub struct TcpListenReader {
    tcp_listener: TcpListener,
    maybe_tcp_stream: Option<TcpStream>,
}

impl TcpListenReader {
    pub async fn new(address: &str) -> Result<Self, Error> {
        info!("TcpListenReader listening on {}", address);
        match TcpListener::bind(address).await {
            Ok(tcp_listener) => Ok(Self {
                tcp_listener,
                maybe_tcp_stream: None,
            }),
            Err(error) => Err(error),
        }
    }

    pub fn get_listen_port(&self) -> Result<SocketAddr, Error> {
        self.tcp_listener.local_addr()
    }
}

impl InputReader for TcpListenReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        if self.maybe_tcp_stream.is_none() {
            info!("Listening for connection");
            match self.tcp_listener.accept().await {
                Ok((tcp_stream, addr)) => {
                    info!("Connection from {} accepted", addr);
                    info!(
                        "TcpStream info: Local Addr:{:?}, Peer Addr:{:?}",
                        tcp_stream.local_addr()?,
                        tcp_stream.peer_addr()?
                    );
                    self.maybe_tcp_stream = Some(tcp_stream);
                }
                Err(error) => {
                    let error_message = format!("Error accepting connection: {}", error);
                    error!("{}", error_message);
                    return Err(Error::new(ErrorKind::ConnectionAborted, error_message));
                }
            }
        }
        let tcp_stream = self.maybe_tcp_stream.as_mut().unwrap();
        let mut vec_bytes = [0; TCP_LISTEN_READ_BUFFER_SIZE];
        info!("Attempting to read the stream");
        match tcp_stream.read(&mut vec_bytes).await {
            Ok(length) => {
                info!("TcpListenReader received {} bytes", length);
                Ok(Bytes::copy_from_slice(&vec_bytes[..length]))
            }
            Err(error) => Err(error),
        }
    }
}
