use crate::datapipe_types::InputReader;
/// "Listen for connection, then pull" Reader for TCP
use bytes::Bytes;
use core::net::SocketAddr;
use log::{error, trace};
use std::io::{Error, ErrorKind};
use tokio::net::{TcpListener, TcpStream};

const TCP_LISTEN_READ_BUFFER_SIZE: usize = 2048;

#[derive(Debug)]
pub struct TcpListenReader {
    tcp_listener: TcpListener,
    maybe_tcp_stream: Option<TcpStream>,
}

impl TcpListenReader {
    pub async fn new(address: &str) -> Result<Self, Error> {
        trace!("TcpListenReader listening on {}", address);
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
            match self.tcp_listener.accept().await {
                Ok((tcp_stream, addr)) => {
                    trace!("Connection from {} accepted", addr);
                    self.maybe_tcp_stream = Some(tcp_stream);
                }
                Err(error) => {
                    let error_message = format!("Error accepting connection: {}", error);
                    error!("{}", error_message);
                    return Err(Error::new(ErrorKind::ConnectionAborted, error_message));
                }
            }
        }
        match &self.maybe_tcp_stream {
            Some(tcp_stream) => {
                tcp_stream.readable().await?;
                let mut vec_bytes = Vec::with_capacity(TCP_LISTEN_READ_BUFFER_SIZE);
                match tcp_stream.try_read(&mut vec_bytes) {
                    Ok(_length) => {
                        trace!("TcpReader received {} bytes", _length);
                        Ok(Bytes::from(vec_bytes))
                    }
                    Err(error) => Err(error),
                }
            }
            None => {
                // this should not be possible
                let error_message = format!("Error TcpStream initialization failure");
                error!("{}", error_message);
                return Err(Error::new(ErrorKind::NotConnected, error_message));
            }
        }
    }
}
