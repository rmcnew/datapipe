use crate::datapipe_types::InputReader;
/// Reader for UDP
use bytes::Bytes;
use log::trace;
use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;

const UDP_READ_BUFFER_SIZE: usize = 65_535;

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
        let (multicast_addr, port) = if let Ok(sock_v4) = address.parse::<SocketAddrV4>() {
            (*sock_v4.ip(), sock_v4.port())
        } else if let Ok(ip_v4) = address.parse::<Ipv4Addr>() {
            (ip_v4, 0)
        } else {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Invalid IPv4 multicast address '{address}': expected 'A.B.C.D' or 'A.B.C.D:PORT'"
                ),
            ));
        };
        if !multicast_addr.is_multicast() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("Address '{multicast_addr}' is not an IPv4 multicast address"),
            ));
        }
        let bind_addr = SocketAddrV4::new(multicast_addr, port);
        match UdpSocket::bind(bind_addr).await {
            Ok(socket) => {
                socket.join_multicast_v4(multicast_addr, Ipv4Addr::UNSPECIFIED)?;
                Ok(UdpReader { socket })
            }
            Err(error) => Err(error),
        }
    }
}

impl InputReader for UdpReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        let mut buf = vec![0u8; UDP_READ_BUFFER_SIZE];
        match self.socket.recv_from(&mut buf).await {
            Ok((length, source_address)) => {
                trace!(
                    "UdpReader received {} bytes from {}",
                    length, source_address
                );
                buf.truncate(length);
                Ok(Bytes::from(buf))
            }
            Err(error) => Err(error),
        }
    }
}
