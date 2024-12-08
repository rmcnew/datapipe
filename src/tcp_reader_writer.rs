/// "Pull-style" Reader for TCP

use bytes::Bytes;
use crate::datapipe_types::{InputReader, OutputWriter};
use log::trace;
use std::io::Error;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;


pub struct TcpReaderWriter {
    tcp_stream: TcpStream,
}

impl TcpReaderWriter {
    pub async fn new(address: &str) -> Result<Self, Error> {
        trace!("TcpReaderWriter connecting to {}", address);
        match TcpStream::connect(address).await {
            Ok(tcp_stream) => Ok(Self { tcp_stream }),
            Err(error) => Err(error),
        }
    }
}

impl InputReader for TcpReaderWriter {
    async fn read(&mut self) -> Result<Bytes, Error> {
        self.tcp_stream.readable().await?;
        let mut vec_bytes = Vec::with_capacity(512);
        match self.tcp_stream.try_read(&mut vec_bytes) {
            Ok(_length) => {
                trace!("TcpReaderWriter received {} bytes", _length);
                Ok(Bytes::from(vec_bytes))
            }
            Err(error) => Err(error),
        }
    }
}

impl OutputWriter for TcpReaderWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.tcp_stream.write_all(bytes).await
    }
}