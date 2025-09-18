use crate::datapipe_types::InputReader;
/// "Listen for connection, then receive" Reader for TLS
use bytes::Bytes;
use core::net::SocketAddr;
use log::{error, info};
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::server::TlsStream;

const TLS_LISTEN_READ_BUFFER_SIZE: usize = 2048;

pub struct TlsListenReader {
    tcp_listener: TcpListener,
    tls_acceptor: TlsAcceptor,
    maybe_tls_stream: Option<TlsStream<TcpStream>>,
}

impl TlsListenReader {
    pub async fn new(address: &str, tls_config: ServerConfig) -> Result<Self, Error> {
        let tls_acceptor = TlsAcceptor::from(Arc::new(tls_config));
        info!("TlsListenReader listening on {}", address);
        match TcpListener::bind(address).await {
            Ok(tcp_listener) => Ok(Self {
                tcp_listener,
                tls_acceptor,
                maybe_tls_stream: None,
            }),
            Err(error) => Err(error),
        }
    }

    pub fn get_listen_port(&self) -> Result<SocketAddr, Error> {
        self.tcp_listener.local_addr()
    }
}

impl InputReader for TlsListenReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        if self.maybe_tls_stream.is_none() {
            info!("Listening for connection");
            match self.tcp_listener.accept().await {
                Ok((tcp_stream, address)) => {
                    info!("TCP connection from {} accepted", address);
                    info!(
                        "TcpStream info: Local Addr:{:?}, Peer Addr:{:?}",
                        tcp_stream.local_addr()?,
                        tcp_stream.peer_addr()?
                    );
                    match self.tls_acceptor.accept(tcp_stream).await {
                        Ok(tls_stream) => {
                            info!("Successfully TCP connection upgraded to TLS");
                            self.maybe_tls_stream = Some(tls_stream);
                        }
                        Err(error) => {
                            let error_message =
                                format!("Error upgrading connection to TLS: {error}");
                            error!("{error_message}");
                            return Err(Error::new(ErrorKind::ConnectionAborted, error_message));
                        }
                    }
                }
                Err(error) => {
                    let error_message = format!("Error accepting TCP connection: {error}");
                    error!("{error_message}");
                    return Err(Error::new(ErrorKind::ConnectionAborted, error_message));
                }
            }
        }
        let tls_stream = self.maybe_tls_stream.as_mut().unwrap();
        let mut vec_bytes = [0; TLS_LISTEN_READ_BUFFER_SIZE];
        info!("Attempting to read the stream");
        match tls_stream.read(&mut vec_bytes).await {
            Ok(length) => {
                info!("TlsListenReader received {} bytes", length);
                Ok(Bytes::copy_from_slice(&vec_bytes[..length]))
            }
            Err(error) => Err(error),
        }
    }
}

impl std::fmt::Debug for TlsListenReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsListenReader")
            .field("tcp_listener", &self.tcp_listener)
            .field("maybe_tls_stream", &self.maybe_tls_stream)
            .finish()
    }
}
