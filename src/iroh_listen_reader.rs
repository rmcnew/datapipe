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
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, channel};
use tokio::task::JoinHandle;

/// Default ALPN protocol identifier used by `datapipe` over Iroh connections.
pub const DEFAULT_IROH_ALPN: &[u8] = b"datapipe/iroh/1";

const IROH_LISTEN_READ_BUFFER_SIZE: usize = 2048;
const IROH_LISTEN_CHANNEL_CAPACITY: usize = 1024;

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

/// Accepts an incoming bi-directional or uni-directional stream from `conn`.
///
/// If one stream direction fails because the peer used the other stream type and already closed
/// the connection, this function falls back to checking the other stream queue before returning
/// an error.
pub(crate) async fn accept_incoming_stream(
    conn: &Connection,
) -> Result<(RecvStream, Option<SendStream>), Error> {
    tokio::select! {
        biased;
        bi_res = conn.accept_bi() => {
            match bi_res {
                Ok((send_stream, recv_stream)) => {
                    info!("IrohListenReader accepted bi-directional stream");
                    Ok((recv_stream, Some(send_stream)))
                }
                Err(bi_err) => {
                    match conn.accept_uni().await {
                        Ok(recv_stream) => {
                            info!("IrohListenReader accepted uni-directional stream");
                            Ok((recv_stream, None))
                        }
                        Err(_) => {
                            let msg = format!("Error accepting Iroh bi-directional stream: {bi_err}");
                            error!("{msg}");
                            Err(Error::new(ErrorKind::ConnectionAborted, msg))
                        }
                    }
                }
            }
        }
        uni_res = conn.accept_uni() => {
            match uni_res {
                Ok(recv_stream) => {
                    info!("IrohListenReader accepted uni-directional stream");
                    Ok((recv_stream, None))
                }
                Err(uni_err) => {
                    match conn.accept_bi().await {
                        Ok((send_stream, recv_stream)) => {
                            info!("IrohListenReader accepted bi-directional stream");
                            Ok((recv_stream, Some(send_stream)))
                        }
                        Err(_) => {
                            let msg = format!("Error accepting Iroh uni-directional stream: {uni_err}");
                            error!("{msg}");
                            Err(Error::new(ErrorKind::ConnectionAborted, msg))
                        }
                    }
                }
            }
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
    data_rx: Receiver<Result<Bytes, Error>>,
    accept_task: JoinHandle<()>,
    stream_finished: bool,
}

impl std::fmt::Debug for IrohListenReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohListenReader")
            .field("endpoint_id", &self.endpoint.id())
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

        let (data_tx, data_rx) = channel(IROH_LISTEN_CHANNEL_CAPACITY);
        let accept_endpoint = endpoint.clone();
        let allowed_peer = config.allowed_peer;

        let accept_task = tokio::spawn(async move {
            let mut accepted_conn: Option<Connection> = None;
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
                        accepted_conn = Some(conn);
                        break;
                    }
                    Err(err) => {
                        warn!("IrohListenReader handshake failed: {err}");
                    }
                }
            }

            let Some(conn) = accepted_conn else {
                let _ = data_tx
                    .send(Err(Error::new(
                        ErrorKind::ConnectionAborted,
                        "Iroh endpoint closed before accepting a connection",
                    )))
                    .await;
                return;
            };

            let (mut recv_stream, mut ack_send_stream) = match accept_incoming_stream(&conn).await {
                Ok(streams) => streams,
                Err(err) => {
                    let _ = data_tx.send(Err(err)).await;
                    return;
                }
            };

            let mut buf = [0u8; IROH_LISTEN_READ_BUFFER_SIZE];
            loop {
                match recv_stream.read(&mut buf).await {
                    Ok(Some(length)) => {
                        if length == 0 {
                            continue;
                        }
                        info!("IrohListenReader received {length} bytes");
                        if data_tx
                            .send(Ok(Bytes::copy_from_slice(&buf[..length])))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(None) => {
                        info!("IrohListenReader reached end of stream");
                        if let Some(mut ack_send) = ack_send_stream.take() {
                            let _ = ack_send.finish();
                            let _ = data_tx.send(Ok(Bytes::new())).await;
                            let _ =
                                tokio::time::timeout(Duration::from_secs(2), ack_send.stopped())
                                    .await;
                        } else {
                            let _ = data_tx.send(Ok(Bytes::new())).await;
                        }
                        break;
                    }
                    Err(err) => {
                        let msg = format!("IrohListenReader read error: {err}");
                        error!("{msg}");
                        let _ = data_tx
                            .send(Err(Error::new(ErrorKind::BrokenPipe, msg)))
                            .await;
                        break;
                    }
                }
            }
        });

        Ok(Self {
            endpoint,
            data_rx,
            accept_task,
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

        match self.data_rx.recv().await {
            Some(Ok(bytes)) => {
                if bytes.is_empty() {
                    self.stream_finished = true;
                }
                Ok(bytes)
            }
            Some(Err(err)) => Err(err),
            None => {
                self.stream_finished = true;
                Ok(Bytes::new())
            }
        }
    }
}
