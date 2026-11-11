//! Iroh peer-to-peer QUIC listener input implementation (`--iroh-listen-input`).
//!
//! [`IrohListenReader`] binds a local [`iroh::Endpoint`], accepts an incoming QUIC connection
//! matching the configured ALPN protocol identifier, and reads bytes from the peer's stream.

use crate::datapipe_types::InputReader;
use bytes::Bytes;
use core::net::SocketAddr;
use iroh::endpoint::{Connection, PortmapperConfig, RecvStream, SendStream, presets};
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMap, RelayMode, RelayUrl, SecretKey};
use log::{error, info, warn};
use std::io::{Error, ErrorKind};
use tokio::sync::mpsc::{Receiver, channel};
use tokio::task::JoinHandle;

/// Default ALPN protocol identifier used by `datapipe` over Iroh connections.
pub const DEFAULT_IROH_ALPN: &[u8] = b"datapipe/iroh/1";

const IROH_LISTEN_READ_BUFFER_SIZE: usize = 2048;

/// Configuration parameters for constructing an [`IrohListenReader`].
///
/// # Examples
/// ```
/// use datapipe::iroh_listen_reader::{DEFAULT_IROH_ALPN, IrohListenConfig};
///
/// let config = IrohListenConfig::default();
/// assert_eq!(config.alpn, DEFAULT_IROH_ALPN.to_vec());
/// assert!(!config.no_relay);
/// ```
#[derive(Debug, Clone)]
pub struct IrohListenConfig {
    /// Optional secret key for the local endpoint. If `None`, a random key is generated.
    pub secret_key: Option<SecretKey>,
    /// Optional local socket address to bind.
    pub bind_addr: Option<SocketAddr>,
    /// Application-Layer Protocol Negotiation (ALPN) identifier.
    pub alpn: Vec<u8>,
    /// Optional custom relay server URL.
    pub relay_url: Option<RelayUrl>,
    /// Disable relay servers and external address discovery (local-only mode).
    pub no_relay: bool,
    /// Optional peer [`EndpointId`] allow-list filter; if set, only this peer may connect.
    pub allowed_peer: Option<EndpointId>,
}

impl PartialEq for IrohListenConfig {
    fn eq(&self, other: &Self) -> bool {
        let keys_eq = match (&self.secret_key, &other.secret_key) {
            (Some(a), Some(b)) => a.to_bytes() == b.to_bytes(),
            (None, None) => true,
            _ => false,
        };
        keys_eq
            && self.bind_addr == other.bind_addr
            && self.alpn == other.alpn
            && self.relay_url == other.relay_url
            && self.no_relay == other.no_relay
            && self.allowed_peer == other.allowed_peer
    }
}

impl Eq for IrohListenConfig {}

impl Default for IrohListenConfig {
    fn default() -> Self {
        Self {
            secret_key: None,
            bind_addr: None,
            alpn: DEFAULT_IROH_ALPN.to_vec(),
            relay_url: None,
            no_relay: false,
            allowed_peer: None,
        }
    }
}

/// Listener-mode Iroh input reader that binds a local [`Endpoint`], accepts an incoming
/// peer connection, and streams incoming bytes into the `datapipe` pipeline.
///
/// # Examples
/// ```no_run
/// use datapipe::iroh_listen_reader::{IrohListenConfig, IrohListenReader};
///
/// # async fn example() -> Result<(), std::io::Error> {
/// let mut config = IrohListenConfig::default();
/// config.no_relay = true;
/// let reader = IrohListenReader::new(config).await?;
/// let _id = reader.endpoint_id();
/// # Ok(())
/// # }
/// ```
pub struct IrohListenReader {
    endpoint: Endpoint,
    conn_rx: Receiver<Result<Connection, Error>>,
    accept_task: JoinHandle<()>,
    connection: Option<Connection>,
    recv_stream: Option<RecvStream>,
    ack_send_stream: Option<SendStream>,
    stream_finished: bool,
}

impl std::fmt::Debug for IrohListenReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohListenReader")
            .field("endpoint_id", &self.endpoint.id())
            .field("has_connection", &self.connection.is_some())
            .field("has_recv_stream", &self.recv_stream.is_some())
            .field("stream_finished", &self.stream_finished)
            .finish()
    }
}

impl Drop for IrohListenReader {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

impl IrohListenReader {
    /// Creates and binds a new [`IrohListenReader`] from the provided [`IrohListenConfig`].
    ///
    /// # Errors
    /// Returns [`std::io::Error`] if the ALPN is empty, the socket bind address is invalid,
    /// or the underlying [`iroh::Endpoint`] fails to bind.
    ///
    /// # Examples
    /// ```no_run
    /// use datapipe::iroh_listen_reader::{IrohListenConfig, IrohListenReader};
    ///
    /// # async fn example() -> Result<(), std::io::Error> {
    /// let config = IrohListenConfig {
    ///     no_relay: true,
    ///     ..Default::default()
    /// };
    /// let reader = IrohListenReader::new(config).await?;
    /// assert!(!reader.local_addrs().is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: IrohListenConfig) -> Result<Self, Error> {
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
            if let Some(relay_url) = config.relay_url {
                b = b.relay_mode(RelayMode::Custom(RelayMap::from(relay_url)));
            }
            b
        };

        if let Some(secret_key) = config.secret_key {
            builder = builder.secret_key(secret_key);
        }

        if let Some(bind_addr) = config.bind_addr {
            builder = builder
                .clear_ip_transports()
                .bind_addr(bind_addr)
                .map_err(|err| {
                    let msg = format!("Invalid Iroh listen bind address {bind_addr}: {err}");
                    error!("{msg}");
                    Error::new(ErrorKind::InvalidInput, msg)
                })?;
        }

        builder = builder.alpns(vec![config.alpn]);

        let endpoint = builder.bind().await.map_err(|err| {
            let msg = format!("Failed to bind Iroh listen endpoint: {err}");
            error!("{msg}");
            Error::new(ErrorKind::AddrNotAvailable, msg)
        })?;

        let endpoint_addr = endpoint.addr();
        let local_addrs: Vec<SocketAddr> = endpoint_addr.ip_addrs().copied().collect();
        info!(
            "IrohListenReader bound with EndpointId: {}, local addresses: {:?}",
            endpoint.id(),
            local_addrs
        );

        let (conn_tx, conn_rx) = channel(1);
        let accept_endpoint = endpoint.clone();
        let allowed_peer = config.allowed_peer;

        let accept_task = tokio::spawn(async move {
            while let Some(incoming) = accept_endpoint.accept().await {
                let accepting = match incoming.accept() {
                    Ok(accepting) => accepting,
                    Err(err) => {
                        warn!(
                            "IrohListenReader incoming connection rejected before handshake: {err}"
                        );
                        continue;
                    }
                };
                match accepting.await {
                    Ok(conn) => {
                        let remote_id = conn.remote_id();
                        if let Some(expected_peer) = allowed_peer
                            && remote_id != expected_peer
                        {
                            warn!(
                                "IrohListenReader rejecting unauthorized peer {remote_id} (expected {expected_peer})"
                            );
                            conn.close(1u32.into(), b"unauthorized peer");
                            continue;
                        }
                        info!("IrohListenReader accepted connection from peer {remote_id}");
                        let _ = conn_tx.send(Ok(conn)).await;
                        return;
                    }
                    Err(err) => {
                        warn!("IrohListenReader handshake failed: {err}");
                    }
                }
            }
            let _ = conn_tx
                .send(Err(Error::new(
                    ErrorKind::ConnectionAborted,
                    "Iroh endpoint closed before accepting a connection",
                )))
                .await;
        });

        Ok(Self {
            endpoint,
            conn_rx,
            accept_task,
            connection: None,
            recv_stream: None,
            ack_send_stream: None,
            stream_finished: false,
        })
    }

    /// Returns the cryptographic [`EndpointId`] (Public Key) of this listener endpoint.
    ///
    /// # Examples
    /// ```no_run
    /// use datapipe::iroh_listen_reader::{IrohListenConfig, IrohListenReader};
    ///
    /// # async fn example() -> Result<(), std::io::Error> {
    /// let reader = IrohListenReader::new(IrohListenConfig {
    ///     no_relay: true,
    ///     ..Default::default()
    /// }).await?;
    /// println!("Listening on EndpointId: {}", reader.endpoint_id());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    /// Returns the full [`EndpointAddr`] (including `EndpointId`, direct IP addresses, and relays)
    /// for this listener endpoint.
    #[must_use]
    pub fn endpoint_addr(&self) -> EndpointAddr {
        self.endpoint.addr()
    }

    /// Returns the list of bound local [`SocketAddr`]s for this listener endpoint.
    #[must_use]
    pub fn local_addrs(&self) -> Vec<SocketAddr> {
        self.endpoint.addr().ip_addrs().copied().collect()
    }
}

impl InputReader for IrohListenReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        if self.stream_finished {
            return Ok(Bytes::new());
        }

        if self.connection.is_none() {
            info!(
                "IrohListenReader ({}) waiting for incoming peer connection",
                self.endpoint.id()
            );
            match self.conn_rx.recv().await {
                Some(Ok(conn)) => {
                    self.connection = Some(conn);
                }
                Some(Err(err)) => return Err(err),
                None => {
                    return Err(Error::new(
                        ErrorKind::ConnectionAborted,
                        "Iroh listener accept channel closed unexpectedly",
                    ));
                }
            }
        }

        if self.recv_stream.is_none() {
            let Some(ref conn) = self.connection else {
                return Err(Error::new(
                    ErrorKind::NotConnected,
                    "Iroh connection not established",
                ));
            };

            tokio::select! {
                bi_res = conn.accept_bi() => {
                    match bi_res {
                        Ok((send_stream, recv_stream)) => {
                            info!("IrohListenReader accepted bi-directional stream");
                            self.ack_send_stream = Some(send_stream);
                            self.recv_stream = Some(recv_stream);
                        }
                        Err(err) => {
                            let msg = format!("Error accepting Iroh bi-directional stream: {err}");
                            error!("{msg}");
                            return Err(Error::new(ErrorKind::ConnectionAborted, msg));
                        }
                    }
                }
                uni_res = conn.accept_uni() => {
                    match uni_res {
                        Ok(recv_stream) => {
                            info!("IrohListenReader accepted uni-directional stream");
                            self.recv_stream = Some(recv_stream);
                        }
                        Err(err) => {
                            let msg = format!("Error accepting Iroh uni-directional stream: {err}");
                            error!("{msg}");
                            return Err(Error::new(ErrorKind::ConnectionAborted, msg));
                        }
                    }
                }
            }
        }

        let Some(ref mut recv_stream) = self.recv_stream else {
            return Err(Error::new(
                ErrorKind::NotConnected,
                "Iroh receive stream not available",
            ));
        };

        let mut buf = [0u8; IROH_LISTEN_READ_BUFFER_SIZE];
        match recv_stream.read(&mut buf).await {
            Ok(Some(length)) => {
                info!("IrohListenReader received {length} bytes");
                Ok(Bytes::copy_from_slice(&buf[..length]))
            }
            Ok(None) => {
                info!("IrohListenReader reached end of stream");
                self.stream_finished = true;
                if let Some(mut ack_send) = self.ack_send_stream.take() {
                    let _ = ack_send.finish();
                }
                Ok(Bytes::new())
            }
            Err(err) => {
                let msg = format!("IrohListenReader read error: {err}");
                error!("{msg}");
                Err(Error::new(ErrorKind::BrokenPipe, msg))
            }
        }
    }
}
