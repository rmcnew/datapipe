use crate::datapipe_types::{InputReader, OutputWriter};
/// "Pull-style" Reader for TCP
use bytes::Bytes;
use log::{info, warn};
use std::io::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const TCP_READ_BUFFER_SIZE: usize = 2048;

#[derive(Debug)]
pub struct TcpReaderWriter {
    tcp_stream: TcpStream,
}

impl TcpReaderWriter {
    pub async fn new(address: &str) -> Result<Self, Error> {
        info!("TcpReaderWriter connecting to {}", address);
        match TcpStream::connect(address).await {
            Ok(tcp_stream) => {
                info!(
                    "TcpStream info: Local Addr:{:?}, Peer Addr:{:?}",
                    tcp_stream.local_addr()?,
                    tcp_stream.peer_addr()?
                );
                Ok(Self { tcp_stream })
            }
            Err(error) => Err(error),
        }
    }
}

impl InputReader for TcpReaderWriter {
    async fn read(&mut self) -> Result<Bytes, Error> {
        let mut buf = [0u8; TCP_READ_BUFFER_SIZE];
        match self.tcp_stream.read(&mut buf).await {
            Ok(length) => {
                info!("TcpReaderWriter received {} bytes", length);
                Ok(Bytes::copy_from_slice(&buf[..length]))
            }
            Err(error) => Err(error),
        }
    }
}

impl OutputWriter for TcpReaderWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if !bytes.is_empty() {
            info!("Trying to write {} bytes", bytes.len());
            self.tcp_stream.write_all(bytes).await?;
            self.tcp_stream.flush().await?;
            info!("TcpReaderWriter successfully wrote the bytes");
        } else {
            warn!("Byte slice was empty; nothing to write");
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Error> {
        self.tcp_stream.flush().await?;
        self.tcp_stream.shutdown().await
    }
}
