/// Writer for UDP output

use crate::datapipe_types::OutputWriter;
use log::{error, trace};
use std::io::Error;
use tokio::net::UdpSocket;

pub struct UdpWriter {
    socket: UdpSocket,
}

impl UdpWriter {
    pub async fn new(address: &str) -> Result<Self, Error> {
        trace!("UdpWriter connecting to {}", address);
        match UdpSocket::bind("0.0.0.0:0").await {
            // let the OS choose the local port
            Ok(socket) => match socket.connect(address).await {
                Ok(()) => Ok(Self { socket }),
                Err(error) => {
                    let error_message = format!("UDP connection error {}: {}", &address, error);
                    error!("{}", error_message);
                    Err(Error::new(error.kind(), error_message))
                }
            },
            Err(error) => {
                let error_message = format!("UDP bind error: {}", error);
                error!("{}", error_message);
                Err(Error::new(error.kind(), error_message))
            }
        }
    }
}

impl OutputWriter for UdpWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match self.socket.send(bytes).await {
            Ok(_size) => Ok(()),
            Err(error) => {
                let error_message = format!("Error writing to UDP: {}", error);
                error!("{}", error_message);
                Err(Error::new(error.kind(), error_message))
            }
        }
    }
}
