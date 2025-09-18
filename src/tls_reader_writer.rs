use crate::datapipe_types::{InputReader, OutputWriter};
/// Reader and Writer for TLS (TCP with TLS encryption)
use bytes::Bytes;
use log::{error, info};
use std::io::{Error, ErrorKind};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::rustls::pki_types::ServerName;

const TLS_READER_WRITER_BUFFER_SIZE: usize = 2048;

#[derive(Debug)]
pub struct TlsReaderWriter {
    tls_stream: TlsStream<TcpStream>,
}

impl TlsReaderWriter {
    pub async fn new(address: &str, tls_config: ClientConfig) -> Result<Self, Error> {
        let tls_connector = TlsConnector::from(Arc::new(tls_config));
        // connect a "basic" TCP stream
        info!("Connecting to TCP address: {}", address);
        match TcpStream::connect(address).await {
            Ok(tcp_stream) => {
                // get the domain name
                let address_domain = get_domain(address);
                match ServerName::try_from(address_domain.to_owned()) {
                    // use the TlsConnector to "upgrade" the TCP stream to TLS
                    Ok(domain) => match tls_connector.connect(domain, tcp_stream).await {
                        Ok(tls_stream) => Ok(Self { tls_stream }),
                        Err(error) => {
                            let error_message = format!("TLS connection error: {}", error);
                            error!("{}", error_message);
                            Err(Error::new(ErrorKind::NotConnected, error_message))
                        }
                    },
                    Err(error) => {
                        let error_message =
                            format!("Invalid DNS name '{}': {}", address_domain, error);
                        error!("{}", error_message);
                        Err(Error::new(ErrorKind::InvalidInput, error_message))
                    }
                }
            }
            Err(error) => {
                let error_message =
                    format!("Error connecting TCP stream for TLS connection: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::NotConnected, error_message))
            }
        }
    }
}

impl InputReader for TlsReaderWriter {
    async fn read(&mut self) -> Result<bytes::Bytes, Error> {
        let mut vec_bytes = Vec::with_capacity(TLS_READER_WRITER_BUFFER_SIZE);
        self.tls_stream.read_exact(&mut vec_bytes).await?;
        Ok(Bytes::from(vec_bytes))
    }
}

impl OutputWriter for TlsReaderWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.tls_stream.write_all(bytes).await
    }
}

#[test]
fn test_get_domain() {
    let value1 = "my.happy.server:1234";
    let domain1 = get_domain(value1);
    assert_eq!(&domain1, "my.happy.server");

    let value2 = "server.with.noport";
    let domain2 = get_domain(value2);
    assert_eq!(&domain2, value2);

    let value3 = "10.222.100.200:10191";
    let domain3 = get_domain(value3);
    assert_eq!(&domain3, "10.222.100.200");

    let value4 = "192.168.32.17";
    let domain4 = get_domain(value4);
    assert_eq!(&domain4, value4);
}

fn get_domain(address: &str) -> String {
    match address.rfind(':') {
        Some(index) => {
            let (domain, _port) = address.split_at(index);
            domain.to_string()
        }
        None => address.to_string(),
    }
}
