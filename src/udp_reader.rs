/// Reader for UDP

use bytes::Bytes;
use crate::datapipe_types::InputReader;
use log::trace;
use std::io::Error;
use tokio::net::UdpSocket;


pub struct UdpReader {
    socket: UdpSocket,
}

impl UdpReader {
    pub async fn new(address: &str) -> Result<Self, Error> {
        trace!("UdpReader listening on {}", address);
        match UdpSocket::bind(address).await {
            Ok(socket) => Ok(Self { socket }),
            Err(error) => Err(error),
        }
    }
}

impl InputReader for UdpReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        let mut vec_bytes = Vec::with_capacity(512);
        match self.socket.recv_buf_from(&mut vec_bytes).await {
            Ok((_length, _source_address)) => {
                trace!(
                    "UdpReader received {} bytes from {}",
                    _length,
                    _source_address
                );
                Ok(Bytes::from(vec_bytes))
            }
            Err(error) => Err(error),
        }
    }
}
