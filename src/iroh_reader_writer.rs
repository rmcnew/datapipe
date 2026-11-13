//! Iroh peer-to-peer QUIC outbound reader (`--iroh-input`) and writer (`--iroh-output`).
//!
//! This module provides [`IrohReader`], [`IrohWriter`], [`IrohConnectConfig`], and helper
//! functions for parsing and formatting Iroh [`EndpointId`]s, [`SecretKey`]s, [`RelayUrl`]s,
//! and direct [`SocketAddr`] lists.

use crate::datapipe_types::{InputReader, OutputWriter};
use crate::iroh_listen_reader::DEFAULT_IROH_ALPN;
use bytes::Bytes;
use core::net::SocketAddr;
use iroh::endpoint::{Connection, PortmapperConfig, RecvStream, SendStream, presets};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, RelayMap, RelayMode, RelayUrl, SecretKey, TransportAddr,
};
use log::{error, info, warn};
use std::fmt::Write as _;
use std::io::{Error, ErrorKind};
use std::str::FromStr;
use std::time::Duration;

const IROH_READ_BUFFER_SIZE: usize = 2048;

/// Formats an [`iroh::SecretKey`] as a 64-character lowercase hexadecimal string.
///
/// # Examples
/// ```
/// use datapipe::iroh_reader_writer::{format_iroh_secret_key_hex, parse_iroh_secret_key};
/// use iroh::SecretKey;
///
/// # fn main() -> Result<(), std::io::Error> {
/// let key = SecretKey::generate();
/// let hex = format_iroh_secret_key_hex(&key);
/// assert_eq!(hex.len(), 64);
/// let parsed = parse_iroh_secret_key(&hex)?;
/// assert_eq!(parsed.to_bytes(), key.to_bytes());
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn format_iroh_secret_key_hex(secret_key: &SecretKey) -> String {
    let bytes = secret_key.to_bytes();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Parses an [`iroh::SecretKey`] from a 64-character hex string (case-insensitive) or base32 string.
///
/// # Errors
/// Returns [`std::io::Error`] with [`ErrorKind::InvalidInput`] if the string is not a valid key.
///
/// # Examples
/// ```
/// use datapipe::iroh_reader_writer::{format_iroh_secret_key_hex, parse_iroh_secret_key};
/// use iroh::SecretKey;
///
/// # fn main() -> Result<(), std::io::Error> {
/// let original = SecretKey::generate();
/// let hex = format_iroh_secret_key_hex(&original);
/// let decoded = parse_iroh_secret_key(&hex)?;
/// assert_eq!(original.public(), decoded.public());
/// # Ok(())
/// # }
/// ```
pub fn parse_iroh_secret_key(input: &str) -> Result<SecretKey, Error> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Iroh secret key must not be empty",
        ));
    }
    let normalized = if trimmed.len() == 64 {
        trimmed.to_ascii_lowercase()
    } else {
        trimmed.to_string()
    };
    SecretKey::from_str(&normalized).map_err(|err| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Invalid Iroh secret key: {err}"),
        )
    })
}

/// Parses an [`iroh::EndpointId`] (`PublicKey`) from a 64-character hex string (case-insensitive)
/// or z-base-32 string.
///
/// # Errors
/// Returns [`std::io::Error`] with [`ErrorKind::InvalidInput`] if the string is not a valid
/// [`EndpointId`].
///
/// # Examples
/// ```
/// use datapipe::iroh_reader_writer::parse_iroh_endpoint_id;
/// use iroh::SecretKey;
///
/// # fn main() -> Result<(), std::io::Error> {
/// let key = SecretKey::generate();
/// let id_str = key.public().to_string();
/// let parsed = parse_iroh_endpoint_id(&id_str)?;
/// assert_eq!(parsed, key.public());
/// # Ok(())
/// # }
/// ```
pub fn parse_iroh_endpoint_id(input: &str) -> Result<EndpointId, Error> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Iroh endpoint ID must not be empty",
        ));
    }
    let normalized = if trimmed.len() == 64 {
        trimmed.to_ascii_lowercase()
    } else {
        trimmed.to_string()
    };
    EndpointId::from_str(&normalized).map_err(|err| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Invalid Iroh endpoint ID '{trimmed}': {err}"),
        )
    })
}

/// Parses an [`iroh::RelayUrl`] from a string slice.
///
/// # Errors
/// Returns [`std::io::Error`] with [`ErrorKind::InvalidInput`] if the URL is invalid.
///
/// # Examples
/// ```
/// use datapipe::iroh_reader_writer::parse_iroh_relay_url;
///
/// # fn main() -> Result<(), std::io::Error> {
/// let relay = parse_iroh_relay_url("https://use1-1.relay.iroh.network.")?;
/// assert!(relay.to_string().starts_with("https://"));
/// # Ok(())
/// # }
/// ```
pub fn parse_iroh_relay_url(input: &str) -> Result<RelayUrl, Error> {
    let trimmed = input.trim();
    RelayUrl::from_str(trimmed).map_err(|err| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Invalid Iroh relay URL '{trimmed}': {err}"),
        )
    })
}

/// Parses a comma- or whitespace-separated list of [`SocketAddr`] values.
///
/// # Errors
/// Returns [`std::io::Error`] with [`ErrorKind::InvalidInput`] if any address token fails to parse.
///
/// # Examples
/// ```
/// use datapipe::iroh_reader_writer::parse_iroh_socket_addrs;
///
/// # fn main() -> Result<(), std::io::Error> {
/// let addrs = parse_iroh_socket_addrs("127.0.0.1:9000, 127.0.0.1:9001")?;
/// assert_eq!(addrs.len(), 2);
/// # Ok(())
/// # }
/// ```
pub fn parse_iroh_socket_addrs(input: &str) -> Result<Vec<SocketAddr>, Error> {
    let mut addrs = Vec::new();
    for token in input
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
    {
        let addr = token.parse::<SocketAddr>().map_err(|err| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Invalid Iroh socket address '{token}': {err}"),
            )
        })?;
        addrs.push(addr);
    }
    Ok(addrs)
}

/// Configuration for connecting outbound to a remote [`iroh::Endpoint`] via [`IrohReader`] or [`IrohWriter`].
///
/// # Examples
/// ```
/// use datapipe::iroh_reader_writer::IrohConnectConfig;
/// use iroh::SecretKey;
///
/// let target_id = SecretKey::generate().public();
/// let config = IrohConnectConfig::new(target_id);
/// assert_eq!(config.target_id, target_id);
/// ```
#[derive(Debug, Clone)]
pub struct IrohConnectConfig {
    /// Remote peer [`EndpointId`] (Public Key) to dial.
    pub target_id: EndpointId,
    /// Optional direct IP socket addresses for the remote peer.
    pub target_addrs: Vec<SocketAddr>,
    /// Optional relay server URL for the remote peer and local endpoint.
    pub relay_url: Option<RelayUrl>,
    /// Disable relay servers and DNS/Pkarr discovery (direct-IP mode).
    pub no_relay: bool,
    /// Optional local [`SecretKey`] for authenticating with the remote peer.
    pub secret_key: Option<SecretKey>,
    /// Application-Layer Protocol Negotiation (ALPN) identifier.
    pub alpn: Vec<u8>,
}

impl PartialEq for IrohConnectConfig {
    fn eq(&self, other: &Self) -> bool {
        let keys_eq = match (&self.secret_key, &other.secret_key) {
            (Some(a), Some(b)) => a.to_bytes() == b.to_bytes(),
            (None, None) => true,
            _ => false,
        };
        self.target_id == other.target_id
            && self.target_addrs == other.target_addrs
            && self.relay_url == other.relay_url
            && self.no_relay == other.no_relay
            && keys_eq
            && self.alpn == other.alpn
    }
}

impl Eq for IrohConnectConfig {}

impl IrohConnectConfig {
    /// Creates a new [`IrohConnectConfig`] targeting `target_id` with default ALPN and relay settings.
    #[must_use]
    pub fn new(target_id: EndpointId) -> Self {
        Self {
            target_id,
            target_addrs: Vec::new(),
            relay_url: None,
            no_relay: false,
            secret_key: None,
            alpn: DEFAULT_IROH_ALPN.to_vec(),
        }
    }

    /// Builds the [`EndpointAddr`] describing the remote peer.
    #[must_use]
    pub fn endpoint_addr(&self) -> EndpointAddr {
        let ip_iter = self.target_addrs.iter().copied().map(TransportAddr::Ip);
        let relay_iter = self.relay_url.clone().into_iter().map(TransportAddr::Relay);
        EndpointAddr::from_parts(self.target_id, relay_iter.chain(ip_iter))
    }
}

async fn establish_iroh_connection(
    config: &IrohConnectConfig,
) -> Result<(Endpoint, Connection), Error> {
    if config.alpn.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Iroh ALPN protocol identifier must not be empty",
        ));
    }

    let mut builder = if config.no_relay {
        Endpoint::builder(presets::Minimal).portmapper_config(PortmapperConfig::Disabled)
    } else {
        let mut b = Endpoint::builder(presets::N0);
        if let Some(ref relay_url) = config.relay_url {
            b = b.relay_mode(RelayMode::Custom(RelayMap::from(relay_url.clone())));
        }
        b
    };

    if let Some(ref secret_key) = config.secret_key {
        builder = builder.secret_key(secret_key.clone());
    }

    let endpoint = builder.bind().await.map_err(|err| {
        let msg = format!("Failed to bind local Iroh client endpoint: {err}");
        error!("{msg}");
        Error::new(ErrorKind::AddrNotAvailable, msg)
    })?;

    let remote_addr = config.endpoint_addr();
    info!(
        "Connecting via Iroh from local EndpointId {} to remote EndpointId {}",
        endpoint.id(),
        config.target_id
    );

    let connection = match endpoint.connect(remote_addr, &config.alpn).await {
        Ok(connection) => connection,
        Err(err) => {
            let msg = format!(
                "Failed to connect to Iroh endpoint {}: {err}",
                config.target_id
            );
            error!("{msg}");
            let _ = tokio::time::timeout(Duration::from_secs(2), endpoint.close()).await;
            return Err(Error::new(ErrorKind::NotConnected, msg));
        }
    };

    Ok((endpoint, connection))
}

/// Outbound pull-style Iroh reader (`--iroh-input`) that dials a remote [`EndpointAddr`]
/// and reads bytes from the QUIC stream.
pub struct IrohReader {
    endpoint: Endpoint,
    connection: Connection,
    recv_stream: Option<RecvStream>,
    ack_send_stream: Option<SendStream>,
    bi_candidate: Option<RecvStream>,
    stream_finished: bool,
}

impl std::fmt::Debug for IrohReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohReader")
            .field("local_id", &self.endpoint.id())
            .field("remote_id", &self.connection.remote_id())
            .field("stream_finished", &self.stream_finished)
            .finish()
    }
}

impl IrohReader {
    /// Connects outbound to the configured remote Iroh endpoint and prepares to read stream data.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if binding the local endpoint, dialing the remote peer, or
    /// opening the initial stream fails.
    pub async fn new(config: IrohConnectConfig) -> Result<Self, Error> {
        let (endpoint, connection) = establish_iroh_connection(&config).await?;

        // Open a bi-directional stream and finish our send half so peers using `accept_bi()`
        // are notified immediately, while also allowing peers that initiate via `open_bi()` or `open_uni()`.
        let bi_candidate = match connection.open_bi().await {
            Ok((mut send_stream, recv_stream)) => {
                let _ = send_stream.finish();
                Some(recv_stream)
            }
            Err(err) => {
                if connection.close_reason().is_some() {
                    let msg = format!("IrohReader connection closed during stream setup: {err}");
                    error!("{msg}");
                    let _ = tokio::time::timeout(Duration::from_secs(2), endpoint.close()).await;
                    return Err(Error::new(ErrorKind::ConnectionAborted, msg));
                }
                log::warn!(
                    "IrohReader initial open_bi failed, falling back to accept_bi/uni: {err}"
                );
                None
            }
        };

        Ok(Self {
            endpoint,
            connection,
            recv_stream: None,
            ack_send_stream: None,
            bi_candidate,
            stream_finished: false,
        })
    }

    /// Returns the local [`EndpointId`] of this reader.
    #[must_use]
    pub fn local_endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }
}

impl InputReader for IrohReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        if self.stream_finished {
            return Ok(Bytes::new());
        }

        let mut buf = [0u8; IROH_READ_BUFFER_SIZE];

        if let Some(ref mut recv_stream) = self.recv_stream {
            return match recv_stream.read(&mut buf).await {
                Ok(Some(length)) => {
                    info!("IrohReader received {length} bytes");
                    Ok(Bytes::copy_from_slice(&buf[..length]))
                }
                Ok(None) => {
                    info!("IrohReader reached end of stream");
                    self.stream_finished = true;
                    if let Some(mut ack_send) = self.ack_send_stream.take() {
                        let _ = ack_send.finish();
                    }
                    Ok(Bytes::new())
                }
                Err(err) => {
                    let msg = format!("IrohReader read error: {err}");
                    error!("{msg}");
                    Err(Error::new(ErrorKind::BrokenPipe, msg))
                }
            };
        }

        // First read: accept whichever stream the remote peer sends data on first,
        // without failing if an unused stream candidate closes.
        let mut bi_candidate = self.bi_candidate.take();
        let mut accept_bi_enabled = true;
        let mut accept_uni_enabled = true;
        let mut saw_clean_eof = false;
        let mut last_err: Option<Error> = None;

        while bi_candidate.is_some() || accept_bi_enabled || accept_uni_enabled {
            tokio::select! {
                biased;
                bi_res = async {
                    match bi_candidate.as_mut() {
                        Some(stream) => stream.read(&mut buf).await,
                        None => std::future::pending().await,
                    }
                }, if bi_candidate.is_some() => {
                    match bi_res {
                        Ok(Some(length)) => {
                            self.recv_stream = bi_candidate;
                            return Ok(Bytes::copy_from_slice(&buf[..length]));
                        }
                        Ok(None) => {
                            saw_clean_eof = true;
                            bi_candidate = None;
                        }
                        Err(err) => {
                            last_err = Some(Error::new(
                                ErrorKind::BrokenPipe,
                                format!("IrohReader bi-stream read error: {err}"),
                            ));
                            bi_candidate = None;
                        }
                    }
                }
                srv_bi_res = self.connection.accept_bi(), if accept_bi_enabled => {
                    match srv_bi_res {
                        Ok((send_stream, mut recv_stream)) => {
                            match recv_stream.read(&mut buf).await {
                                Ok(Some(length)) => {
                                    self.ack_send_stream = Some(send_stream);
                                    self.recv_stream = Some(recv_stream);
                                    return Ok(Bytes::copy_from_slice(&buf[..length]));
                                }
                                Ok(None) => {
                                    let mut send_stream = send_stream;
                                    let _ = send_stream.finish();
                                    self.stream_finished = true;
                                    return Ok(Bytes::new());
                                }
                                Err(err) => {
                                    last_err = Some(Error::new(
                                        ErrorKind::BrokenPipe,
                                        format!("IrohReader server bi-stream read error: {err}"),
                                    ));
                                    accept_bi_enabled = false;
                                }
                            }
                        }
                        Err(err) => {
                            last_err = Some(Error::new(
                                ErrorKind::ConnectionAborted,
                                format!("IrohReader accept_bi error: {err}"),
                            ));
                            accept_bi_enabled = false;
                        }
                    }
                }
                uni_res = self.connection.accept_uni(), if accept_uni_enabled => {
                    match uni_res {
                        Ok(mut uni_recv) => {
                            match uni_recv.read(&mut buf).await {
                                Ok(Some(length)) => {
                                    self.recv_stream = Some(uni_recv);
                                    return Ok(Bytes::copy_from_slice(&buf[..length]));
                                }
                                Ok(None) => {
                                    self.stream_finished = true;
                                    return Ok(Bytes::new());
                                }
                                Err(err) => {
                                    last_err = Some(Error::new(
                                        ErrorKind::BrokenPipe,
                                        format!("IrohReader uni-stream read error: {err}"),
                                    ));
                                    accept_uni_enabled = false;
                                }
                            }
                        }
                        Err(err) => {
                            last_err = Some(Error::new(
                                ErrorKind::ConnectionAborted,
                                format!("IrohReader accept_uni error: {err}"),
                            ));
                            accept_uni_enabled = false;
                        }
                    }
                }
            }
        }

        let is_graceful_app_close = match self.connection.close_reason() {
            None => true,
            Some(iroh::endpoint::ConnectionError::ApplicationClosed(ref frame)) => {
                frame.error_code.into_inner() == 0
            }
            _ => false,
        };
        if saw_clean_eof && (last_err.is_none() || is_graceful_app_close) {
            self.stream_finished = true;
            return Ok(Bytes::new());
        }

        Err(last_err.unwrap_or_else(|| {
            Error::new(
                ErrorKind::ConnectionAborted,
                "IrohReader connection closed without receiving a stream",
            )
        }))
    }
}

/// Outbound Iroh writer (`--iroh-output`) that connects to a remote [`EndpointAddr`]
/// and streams bytes over QUIC.
pub struct IrohWriter {
    endpoint: Endpoint,
    connection: Connection,
    send_stream: SendStream,
    ack_recv_stream: Option<RecvStream>,
    closed: bool,
}

impl std::fmt::Debug for IrohWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohWriter")
            .field("local_id", &self.endpoint.id())
            .field("remote_id", &self.connection.remote_id())
            .field("closed", &self.closed)
            .finish()
    }
}

impl IrohWriter {
    /// Connects to the remote Iroh endpoint described by `config` and opens an outbound stream.
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if binding the local endpoint, connecting to the peer, or
    /// opening the QUIC stream fails.
    pub async fn new(config: IrohConnectConfig) -> Result<Self, Error> {
        let (endpoint, connection) = establish_iroh_connection(&config).await?;
        let (send_stream, ack_recv_stream) = match connection.open_bi().await {
            Ok(streams) => streams,
            Err(err) => {
                let msg = format!("Failed to open Iroh QUIC stream: {err}");
                error!("{msg}");
                let _ = tokio::time::timeout(Duration::from_secs(2), endpoint.close()).await;
                return Err(Error::new(ErrorKind::NotConnected, msg));
            }
        };

        Ok(Self {
            endpoint,
            connection,
            send_stream,
            ack_recv_stream: Some(ack_recv_stream),
            closed: false,
        })
    }

    /// Returns the local [`EndpointId`] of this writer.
    #[must_use]
    pub fn local_endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }
}

impl OutputWriter for IrohWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if !bytes.is_empty() {
            info!("IrohWriter writing {} bytes", bytes.len());
            self.send_stream.write_all(bytes).await.map_err(|err| {
                let msg = format!("IrohWriter write error: {err}");
                error!("{msg}");
                Error::new(ErrorKind::BrokenPipe, msg)
            })?;
        } else {
            warn!("Byte slice was empty; nothing for IrohWriter to write");
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<(), Error> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;

        if let Err(err) = self.send_stream.finish() {
            warn!("IrohWriter finish stream warning: {err}");
        }

        // Wait for the remote peer to finish its half of the bidirectional stream (or reset/close)
        // after reading all bytes, ensuring we never send `CONNECTION_CLOSE` before the peer
        // application has accepted and drained the stream.
        if let Some(mut ack_recv) = self.ack_recv_stream.take() {
            let _ = tokio::time::timeout(Duration::from_secs(10), ack_recv.read_to_end(16)).await;
        } else {
            let _ = tokio::time::timeout(Duration::from_secs(5), self.send_stream.stopped()).await;
        }

        self.connection.close(0u32.into(), b"done");
        let _ = tokio::time::timeout(Duration::from_secs(2), self.endpoint.close()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iroh_listen_reader::{IrohListenConfig, IrohListenReader};

    #[test]
    fn test_secret_key_and_endpoint_id_parsing() -> Result<(), Error> {
        let secret = SecretKey::generate();
        let hex = format_iroh_secret_key_hex(&secret);
        assert_eq!(hex.len(), 64);

        let parsed_lower = parse_iroh_secret_key(&hex)?;
        let parsed_upper = parse_iroh_secret_key(&hex.to_ascii_uppercase())?;
        assert_eq!(parsed_lower.to_bytes(), secret.to_bytes());
        assert_eq!(parsed_upper.to_bytes(), secret.to_bytes());

        let id = secret.public();
        let id_str = id.to_string();
        assert_eq!(parse_iroh_endpoint_id(&id_str)?, id);
        assert_eq!(parse_iroh_endpoint_id(&id_str.to_ascii_uppercase())?, id);

        assert!(parse_iroh_secret_key("").is_err());
        assert!(parse_iroh_secret_key("invalid_key").is_err());
        assert!(parse_iroh_endpoint_id("").is_err());
        assert!(parse_iroh_endpoint_id("invalid_id").is_err());
        Ok(())
    }

    #[test]
    fn test_socket_addrs_parsing() -> Result<(), Error> {
        let addrs = parse_iroh_socket_addrs("127.0.0.1:8080, 127.0.0.1:8081  [::1]:9000")?;
        assert_eq!(addrs.len(), 3);
        assert!(parse_iroh_socket_addrs("not_an_addr").is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_iroh_invalid_empty_alpn() {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let listen_cfg = IrohListenConfig {
            alpn: Vec::new(),
            no_relay: true,
            ..Default::default()
        };
        assert!(IrohListenReader::new(listen_cfg).await.is_err());

        let mut connect_cfg = IrohConnectConfig::new(SecretKey::generate().public());
        connect_cfg.alpn = Vec::new();
        connect_cfg.no_relay = true;
        assert!(IrohWriter::new(connect_cfg.clone()).await.is_err());
        assert!(IrohReader::new(connect_cfg).await.is_err());
    }

    #[tokio::test]
    async fn test_iroh_listen_and_writer_loopback() -> Result<(), Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let listener_key = SecretKey::generate();
        let listener_id = listener_key.public();

        let listen_cfg = IrohListenConfig {
            secret_key: Some(listener_key),
            bind_addr: Some("127.0.0.1:0".parse().map_err(Error::other)?),
            alpn: DEFAULT_IROH_ALPN.to_vec(),
            relay_url: None,
            no_relay: true,
            allowed_peer: None,
        };

        let mut reader = IrohListenReader::new(listen_cfg).await?;
        assert_eq!(reader.endpoint_id(), listener_id);
        let listen_addrs = reader.local_addrs();
        assert!(!listen_addrs.is_empty());

        let mut connect_cfg = IrohConnectConfig::new(listener_id);
        connect_cfg.target_addrs = listen_addrs;
        connect_cfg.no_relay = true;

        let mut writer = IrohWriter::new(connect_cfg).await?;
        let payload = b"hello over iroh quic stream!";
        writer.write(payload).await?;
        writer.close().await?;

        let mut received = Vec::new();
        loop {
            let chunk = reader.read().await?;
            if chunk.is_empty() {
                break;
            }
            received.extend_from_slice(&chunk);
        }

        assert_eq!(received, payload);
        Ok(())
    }

    #[tokio::test]
    async fn test_iroh_listen_and_writer_concurrent_multi_chunk() -> Result<(), Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let listener_key = SecretKey::generate();
        let listener_id = listener_key.public();

        let listen_cfg = IrohListenConfig {
            secret_key: Some(listener_key),
            bind_addr: Some("127.0.0.1:0".parse().map_err(Error::other)?),
            alpn: DEFAULT_IROH_ALPN.to_vec(),
            relay_url: None,
            no_relay: true,
            allowed_peer: None,
        };

        let mut reader = IrohListenReader::new(listen_cfg).await?;
        let listen_addrs = reader.local_addrs();

        let reader_handle = tokio::spawn(async move {
            let mut total = Vec::new();
            loop {
                let chunk = reader.read().await?;
                if chunk.is_empty() {
                    break;
                }
                total.extend_from_slice(&chunk);
            }
            Ok::<Vec<u8>, Error>(total)
        });

        let mut connect_cfg = IrohConnectConfig::new(listener_id);
        connect_cfg.target_addrs = listen_addrs;
        connect_cfg.no_relay = true;

        let mut writer = IrohWriter::new(connect_cfg).await?;
        let mut expected = Vec::new();
        for idx in 0..32u32 {
            let block = format!("chunk-{idx:04}-payload-data\n").repeat(32);
            writer.write(block.as_bytes()).await?;
            expected.extend_from_slice(block.as_bytes());
        }
        writer.close().await?;

        let received = reader_handle.await.map_err(Error::other)??;
        assert_eq!(received.len(), expected.len());
        assert_eq!(received, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_iroh_listen_allowed_peer_filter() -> Result<(), Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let listener_key = SecretKey::generate();
        let listener_id = listener_key.public();

        let authorized_client_key = SecretKey::generate();
        let authorized_client_id = authorized_client_key.public();

        let listen_cfg = IrohListenConfig {
            secret_key: Some(listener_key),
            bind_addr: Some("127.0.0.1:0".parse().map_err(Error::other)?),
            alpn: DEFAULT_IROH_ALPN.to_vec(),
            relay_url: None,
            no_relay: true,
            allowed_peer: Some(authorized_client_id),
        };

        let mut reader = IrohListenReader::new(listen_cfg).await?;
        let listen_addrs = reader.local_addrs();

        // Connect with the authorized peer key and verify transfer succeeds.
        let mut auth_cfg = IrohConnectConfig::new(listener_id);
        auth_cfg.target_addrs = listen_addrs;
        auth_cfg.no_relay = true;
        auth_cfg.secret_key = Some(authorized_client_key);

        let mut writer = IrohWriter::new(auth_cfg).await?;
        let expected = b"authorized peer payload";
        writer.write(expected).await?;
        writer.close().await?;

        let mut received = Vec::new();
        loop {
            let chunk = reader.read().await?;
            if chunk.is_empty() {
                break;
            }
            received.extend_from_slice(&chunk);
        }
        assert_eq!(received, expected);
        Ok(())
    }

    #[tokio::test]
    async fn test_iroh_reader_outbound_pull() -> Result<(), Error> {
        let _ = rustls::crypto::ring::default_provider().install_default();

        let server_key = SecretKey::generate();
        let server_id = server_key.public();
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().map_err(Error::other)?;

        let server_endpoint = Endpoint::builder(presets::Minimal)
            .portmapper_config(PortmapperConfig::Disabled)
            .secret_key(server_key)
            .clear_ip_transports()
            .bind_addr(bind_addr)
            .map_err(Error::other)?
            .alpns(vec![DEFAULT_IROH_ALPN.to_vec()])
            .bind()
            .await
            .map_err(Error::other)?;

        let server_addrs: Vec<SocketAddr> = server_endpoint.addr().ip_addrs().copied().collect();
        let expected_payload = b"outbound pull payload from iroh server";

        let server_task = tokio::spawn(async move {
            if let Some(incoming) = server_endpoint.accept().await
                && let Ok(accepting) = incoming.accept()
                && let Ok(conn) = accepting.await
                && let Ok((mut send_stream, mut recv_stream)) = conn.accept_bi().await
            {
                let _ = recv_stream.read_to_end(16).await;
                let _ = send_stream.write_all(expected_payload).await;
                let _ = send_stream.finish();
                let _ = tokio::time::timeout(Duration::from_secs(2), send_stream.stopped()).await;
                conn.close(0u32.into(), b"done");
            }
            let _ = tokio::time::timeout(Duration::from_secs(2), server_endpoint.close()).await;
        });

        let mut connect_cfg = IrohConnectConfig::new(server_id);
        connect_cfg.target_addrs = server_addrs;
        connect_cfg.no_relay = true;

        let mut reader = IrohReader::new(connect_cfg).await?;
        let mut received = Vec::new();
        loop {
            let chunk = reader.read().await?;
            if chunk.is_empty() {
                break;
            }
            received.extend_from_slice(&chunk);
        }

        let _ = server_task.await;
        assert_eq!(received, expected_payload);
        Ok(())
    }
}
