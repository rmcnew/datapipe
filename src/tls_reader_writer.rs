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
        let mut buf = [0u8; TLS_READER_WRITER_BUFFER_SIZE];
        let length = self.tls_stream.read(&mut buf).await?;
        Ok(Bytes::copy_from_slice(&buf[..length]))
    }
}

impl OutputWriter for TlsReaderWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.tls_stream.write_all(bytes).await?;
        self.tls_stream.flush().await
    }

    async fn close(&mut self) -> Result<(), Error> {
        self.tls_stream.flush().await?;
        self.tls_stream.shutdown().await
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

    assert_eq!(get_domain("[::1]:443"), "::1");
    assert_eq!(get_domain("[2001:db8::1]"), "2001:db8::1");
    assert_eq!(get_domain("::1"), "::1");
}

fn get_domain(address: &str) -> String {
    if let Some(after_open) = address.strip_prefix('[')
        && let Some((host, _rest)) = after_open.split_once(']')
    {
        return host.to_string();
    }
    if address.parse::<std::net::IpAddr>().is_ok() {
        return address.to_string();
    }
    if let Some((host, port)) = address.rsplit_once(':')
        && !host.contains(':')
        && port.chars().all(|c| c.is_ascii_digit())
    {
        return host.to_string();
    }
    address.to_string()
}

#[cfg(test)]
mod tests {
    use super::get_domain;

    #[test]
    fn test_get_domain_ipv4_ipv6_and_hostname() {
        assert_eq!(get_domain("[::1]:443"), "::1");
        assert_eq!(get_domain("[2001:db8::1]"), "2001:db8::1");
        assert_eq!(get_domain("::1"), "::1");
        assert_eq!(get_domain("127.0.0.1:8443"), "127.0.0.1");
        assert_eq!(get_domain("example.com:8443"), "example.com");
        assert_eq!(get_domain("example.com"), "example.com");
    }
}
