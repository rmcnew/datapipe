use crate::datapipe_types::InputReader;
/// Reader for UDP
use bytes::Bytes;
use log::trace;
use std::io::Error;
use std::net::Ipv4Addr;
use tokio::net::UdpSocket;

#[derive(Debug)]
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

    pub async fn new_multicast(address: &str) -> Result<UdpReader, Error> {
        trace!("UdpReader multicast listening on {}", address);
        let addr = address.parse::<Ipv4Addr>().unwrap();
        let addr_port_zero = format!("{}:0", addr);
        match UdpSocket::bind(addr_port_zero).await {
            Ok(socket) => {
                socket.join_multicast_v4(addr, Ipv4Addr::UNSPECIFIED)?;
                Ok(UdpReader { socket })
            }
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
                    _length, _source_address
                );
                Ok(Bytes::from(vec_bytes))
            }
            Err(error) => Err(error),
        }
    }
}
