use crate::datapipe_types::{DatapipeError, EncryptionKey, error_root_cause};
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::file_reader::FileReader;
use crate::file_writer::FileWriter;
use crate::http_reader::HttpReader;
use crate::http_writer::HttpWriter;
use crate::https_reader::HttpsReader;
use crate::https_writer::HttpsWriter;
use crate::iroh_listen_reader::{DEFAULT_IROH_ALPN, IrohListenConfig, IrohListenReader};
use crate::iroh_reader_writer::{
    IrohConnectConfig, IrohReader, IrohWriter, parse_iroh_endpoint_id, parse_iroh_relay_url,
    parse_iroh_secret_key, parse_iroh_socket_addrs,
};
use crate::parameters::Parameters;
use crate::reader::Reader;
use crate::stdin_reader::StdinReader;
use crate::stdout_writer::StdoutWriter;
use crate::tcp_listen_reader::TcpListenReader;
use crate::tcp_reader_writer::TcpReaderWriter;
use crate::tls_listen_reader::TlsListenReader;
use crate::tls_reader_writer::TlsReaderWriter;
use crate::udp_reader::UdpReader;
use crate::udp_writer::UdpWriter;
use crate::writer::Writer;
use clap::{Args, Parser};
use log::{error, info};
use rcgen::{CertifiedKey, KeyPair, generate_simple_self_signed};
use reqwest::{Certificate, Identity, tls::CertificateRevocationList};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer, PrivateSec1KeyDer,
    ServerName, UnixTime,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls::rustls::client::WantsClientCert;
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::server::WebPkiClientVerifier;
use tokio_rustls::rustls::{
    ClientConfig, ConfigBuilder, DigitallySignedStruct, RootCertStore, ServerConfig,
    SignatureScheme,
};
use webpki_roots::TLS_SERVER_ROOTS;

/// Choose one input source
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = false)]
#[serde(default)]
pub struct InputArgs {
    /// read data from a file
    #[arg(long = "file-input")]
    pub file_input: Option<PathBuf>,
    /// read data from an HTTP URL; requires --http-input-rate
    #[arg(long = "http-input")]
    pub http_input: Option<String>,
    /// read data from an HTTPS URL; requires --https-input-rate and optionally server and client certificates
    #[arg(long = "https-input")]
    pub https_input: Option<String>,
    /// read data from STDIN
    #[arg(long = "stdin-input", default_value_t = false)]
    pub stdin_input: bool,
    /// read data from a TCP address
    #[arg(long = "tcp-input")]
    pub tcp_input: Option<String>,
    /// open a local port to listen and receive data using a TCP connection
    #[arg(long = "tcp-listen-input")]
    pub tcp_listen_input: Option<String>,
    /// read data from a TLS address; may need to configure server and client certificates
    #[arg(long = "tls-input")]
    pub tls_input: Option<String>,
    /// open a local port to listen and receive data using a TLS connection; may need to configure
    /// server certificates
    #[arg(long = "tls-listen-input")]
    pub tls_listen_input: Option<String>,
    /// read data from a UDP address
    #[arg(long = "udp-input")]
    pub udp_input: Option<String>,
    /// read data from a UDP multicast address
    #[arg(long = "udp-multicast-input")]
    pub udp_multicast_input: Option<String>,
    /// read data from an Iroh endpoint by EndpointId (64-character hex or z-base-32 public key)
    #[arg(long = "iroh-input")]
    pub iroh_input: Option<String>,
    /// open a local Iroh endpoint to listen and receive data from a connecting peer
    #[arg(long = "iroh-listen-input", default_value_t = false)]
    pub iroh_listen_input: bool,
}

/// Additional parameters for HTTP input
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct HttpInputArgs {
    /// read rate in milliseconds, how often should the input web address be polled?
    #[arg(long = "http-input-rate")]
    pub http_input_rate: Option<u64>,
}

/// Additional parameters for HTTPS input
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct HttpsInputArgs {
    /// read rate in milliseconds, how often should the input web address be polled?
    #[arg(long = "https-input-rate")]
    pub https_input_rate: Option<u64>,
    /// path to custom root certificate file to use in PEM bundle format
    #[arg(long = "https-input-root-certificates")]
    pub https_input_root_certificates: Option<PathBuf>,
    /// path to custom certificate revocation list file to use in PEM format
    #[arg(long = "https-input-certificate-revocation-list")]
    pub https_input_certificate_revocation_list: Option<PathBuf>,
    /// path to client's custom private key and X509 certificate in PEM format.  Private key must be RSA, SEC1 Elliptic Curve, or PKCS#8.
    #[arg(long = "https-input-client-identity")]
    pub https_input_client_identity: Option<PathBuf>,
    /// DANGER! Do not validate hostnames in HTTPS setup.  Use with caution.  DANGER!
    #[arg(long = "https-input-allow-invalid-hostnames", default_value_t = false)]
    pub https_input_allow_invalid_hostnames: bool,
    /// DANGER! Do not validate certificates in HTTPS setup.  Use with caution. DANGER!
    #[arg(
        long = "https-input-allow-invalid-certificates",
        default_value_t = false
    )]
    pub https_input_allow_invalid_certificates: bool,
}

/// Additional parameters needed for TLS input
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct TlsInputArgs {
    /// path to custom TLS certificate chain file to use (PEM or DER format)
    #[arg(long = "tls-input-cert-chain")]
    pub tls_input_cert_chain: Option<PathBuf>,
    /// path to custom TLS client key to use (PEM or DER PKCS#1, PKCS#8, or SEC1)
    #[arg(long = "tls-input-client-key")]
    pub tls_input_client_key: Option<PathBuf>,
    /// path to custom Certificate Authority to use instead of web root CAs (PEM or DER format)
    #[arg(long = "tls-input-root-ca")]
    pub tls_input_root_ca: Option<PathBuf>,
    /// DANGER! Do not validate server identity.  Use with caution. DANGER!
    #[arg(long = "tls-input-skip-server-verify", default_value_t = false)]
    pub tls_input_skip_server_verify: bool,
}

/// Additional parameters needed for TLS listen input
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct TlsListenInputArgs {
    /// path to custom TLS certificate chain file to use (PEM or DER format)
    #[arg(long = "tls-listen-input-cert-chain")]
    pub tls_listen_input_cert_chain: Option<PathBuf>,
    /// path to custom TLS server key to use (PEM or DER PKCS#1, PKCS#8, or SEC1)
    #[arg(long = "tls-listen-input-server-key")]
    pub tls_listen_input_server_key: Option<PathBuf>,
    /// DANGER! Do not validate client identity.  Use with caution. DANGER!
    #[arg(long = "tls-listen-input-skip-client-verify", default_value_t = false)]
    pub tls_listen_input_skip_client_verify: bool,
    /// Instead of providing a certificate chain and private key, generate a self-signed certificate and private key
    #[arg(
        long = "tls-listen-input-generate-self-signed",
        default_value_t = false
    )]
    pub tls_listen_input_generate_self_signed: bool,
}

/// Additional parameters needed for Iroh outbound input (`--iroh-input`)
///
/// # Examples
///
/// ```
/// use datapipe::args::IrohInputArgs;
///
/// let args = IrohInputArgs::default();
/// assert!(args.is_empty());
/// ```
#[derive(Args, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct IrohInputArgs {
    /// comma-separated direct IP socket addresses (`ip:port`) for the target Iroh endpoint
    #[arg(long = "iroh-input-addrs")]
    pub iroh_input_addrs: Option<String>,
    /// custom relay server URL (`http://` or `https://`) for reaching the target Iroh endpoint
    #[arg(long = "iroh-input-relay-url")]
    pub iroh_input_relay_url: Option<String>,
    /// custom ALPN protocol identifier (defaults to `datapipe/iroh/1`)
    #[arg(long = "iroh-input-alpn")]
    pub iroh_input_alpn: Option<String>,
    /// local 32-byte Ed25519 secret key in 64-character hex (or z-base-32) format
    #[arg(long = "iroh-input-secret-key")]
    pub iroh_input_secret_key: Option<String>,
    /// disable default relay servers (direct IP transport only)
    #[arg(long = "iroh-input-no-relay", default_value_t = false)]
    pub iroh_input_no_relay: bool,
}

impl std::fmt::Debug for IrohInputArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohInputArgs")
            .field("iroh_input_addrs", &self.iroh_input_addrs)
            .field("iroh_input_relay_url", &self.iroh_input_relay_url)
            .field("iroh_input_alpn", &self.iroh_input_alpn)
            .field(
                "iroh_input_secret_key",
                &self.iroh_input_secret_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("iroh_input_no_relay", &self.iroh_input_no_relay)
            .finish()
    }
}

/// Additional parameters needed for Iroh listen input (`--iroh-listen-input`)
///
/// # Examples
///
/// ```
/// use datapipe::args::IrohListenInputArgs;
///
/// let args = IrohListenInputArgs::default();
/// assert!(args.is_empty());
/// ```
#[derive(Args, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct IrohListenInputArgs {
    /// optional local UDP bind address (`ip:port`) for the Iroh endpoint
    #[arg(long = "iroh-listen-input-bind-addr")]
    pub iroh_listen_input_bind_addr: Option<String>,
    /// local 32-byte Ed25519 secret key in 64-character hex (or z-base-32) format for deterministic EndpointId
    #[arg(long = "iroh-listen-input-secret-key")]
    pub iroh_listen_input_secret_key: Option<String>,
    /// custom ALPN protocol identifier (defaults to `datapipe/iroh/1`)
    #[arg(long = "iroh-listen-input-alpn")]
    pub iroh_listen_input_alpn: Option<String>,
    /// custom relay server URL (`http://` or `https://`) for the listening Iroh endpoint
    #[arg(long = "iroh-listen-input-relay-url")]
    pub iroh_listen_input_relay_url: Option<String>,
    /// optional remote peer EndpointId to restrict inbound connections to
    #[arg(long = "iroh-listen-input-allowed-peer")]
    pub iroh_listen_input_allowed_peer: Option<String>,
    /// disable default relay servers (direct IP transport only)
    #[arg(long = "iroh-listen-input-no-relay", default_value_t = false)]
    pub iroh_listen_input_no_relay: bool,
}

impl std::fmt::Debug for IrohListenInputArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohListenInputArgs")
            .field(
                "iroh_listen_input_bind_addr",
                &self.iroh_listen_input_bind_addr,
            )
            .field(
                "iroh_listen_input_secret_key",
                &self
                    .iroh_listen_input_secret_key
                    .as_ref()
                    .map(|_| "[REDACTED]"),
            )
            .field("iroh_listen_input_alpn", &self.iroh_listen_input_alpn)
            .field(
                "iroh_listen_input_relay_url",
                &self.iroh_listen_input_relay_url,
            )
            .field(
                "iroh_listen_input_allowed_peer",
                &self.iroh_listen_input_allowed_peer,
            )
            .field(
                "iroh_listen_input_no_relay",
                &self.iroh_listen_input_no_relay,
            )
            .finish()
    }
}

/// Additional parameters for decrypting input
#[derive(Args, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct DecryptionArgs {
    /// decryption key to use after reading data (`dpk1:...`, `hex:...`, or legacy 51-byte key)
    #[arg(long = "decrypt")]
    pub decryption_key: Option<String>,
    /// allow legacy v1 streams without authenticated EOF framing (UNSAFE against truncation)
    #[arg(long = "decrypt-allow-legacy", default_value_t = false)]
    pub decrypt_allow_legacy: bool,
}

impl std::fmt::Debug for DecryptionArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecryptionArgs")
            .field(
                "decryption_key",
                &self.decryption_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("decrypt_allow_legacy", &self.decrypt_allow_legacy)
            .finish()
    }
}

/// Additional parameters for encrypting output
#[derive(Args, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = false)]
#[serde(default)]
pub struct EncryptionArgs {
    /// encryption key to use before writing data (`dpk1:...`, `hex:...`, or legacy 51-byte key)
    #[arg(long = "encrypt")]
    pub encryption_key: Option<String>,
    /// generate a full-entropy encryption key (printed to stderr)
    #[arg(long = "encrypt-generate-key", default_value_t = false)]
    pub generate_encryption_key: bool,
}

impl std::fmt::Debug for EncryptionArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionArgs")
            .field(
                "encryption_key",
                &self.encryption_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("generate_encryption_key", &self.generate_encryption_key)
            .finish()
    }
}

/// Choose one or more output destinations
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct OutputArgs {
    /// write data to a file
    #[arg(long = "file-output")]
    pub file_output: Option<PathBuf>,
    /// write data to an HTTP URL; requires http-output-rate and optionally http-output-delimiter and http-output-include-delimiter
    #[arg(long = "http-output")]
    pub http_output: Option<String>,
    /// write data to a HTTPS URL; requires https-output-rate and optionally https-output-delimiter, https-output-include-delimiter, and custom server / client certificates
    #[arg(long = "https-output")]
    pub https_output: Option<String>,
    /// write data to STDOUT
    #[arg(long = "stdout-output", default_value_t = false)]
    pub stdout_output: bool,
    /// write data to a TCP address
    #[arg(long = "tcp-output")]
    pub tcp_output: Option<String>,
    /// write data to a TLS URL; may need to configure server and client certificates
    #[arg(long = "tls-output")]
    pub tls_output: Option<String>,
    /// write data to UDP address
    #[arg(long = "udp-output")]
    pub udp_output: Option<String>,
    /// write data to an Iroh endpoint by EndpointId (64-character hex or z-base-32 public key)
    #[arg(long = "iroh-output")]
    pub iroh_output: Option<String>,
}

/// Additional parameters needed for HTTP output
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct HttpOutputArgs {
    /// write rate in milliseconds, how often should data be sent to the output web address?
    #[arg(long = "http-output-rate")]
    pub http_output_rate: Option<u64>,
    /// this delimiter is used to group the output into one or more 'segments' that will be sent in each request; defaults to newline
    #[arg(long = "http-output-delimiter")]
    pub http_output_delimiter: Option<Vec<u8>>,
    /// should the delimiter be included with the segment that preceeds it?  defaults to true
    #[arg(long = "http-output-include-delimiter", default_value_t = true)]
    pub http_output_include_delimiter: bool,
}

/// Additional parameters for HTTPS output
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct HttpsOutputArgs {
    /// write rate in milliseconds, how often should data be sent to the output web address?
    #[arg(long = "https-output-rate")]
    pub https_output_rate: Option<u64>,
    /// this delimiter is used to group the output into one or more 'segments' that will be sent in each request; defaults to newline
    #[arg(long = "https-output-delimiter")]
    pub https_output_delimiter: Option<Vec<u8>>,
    /// should the delimiter be included with the segment that preceeds it?  defaults to true
    #[arg(long = "https-output-include-delimiter")]
    pub https_output_include_delimiter: Option<bool>,
    /// path to custom root certificate file to use in PEM bundle format
    #[arg(long = "https-output-root-certificates")]
    pub https_output_root_certificates: Option<PathBuf>,
    /// path to custom certificate revocation list file to use in PEM format
    #[arg(long = "https-output-certificate-revocation-list")]
    pub https_output_certificate_revocation_list: Option<PathBuf>,
    /// path to client's custom private key and X509 certificate in PEM format.  Private key must be in RSA, SEC1 Elliptic Curve, or PKCS#8 format.
    #[arg(long = "https-output-client-identity")]
    pub https_output_client_identity: Option<PathBuf>,
    /// DANGER! Do not validate hostnames in HTTPS setup.  Use with caution.  DANGER!
    #[arg(long = "https-output-allow-invalid-hostnames", default_value_t = false)]
    pub https_output_allow_invalid_hostnames: bool,
    /// DANGER! Do not validate certificates in HTTPS setup.  Use with caution. DANGER!
    #[arg(
        long = "https-output-allow-invalid-certificates",
        default_value_t = false
    )]
    pub https_output_allow_invalid_certificates: bool,
}

/// Additional parameters needed for TLS output
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct TlsOutputArgs {
    /// path to custom TLS certificate chain file to use (PEM or DER format)
    #[arg(long = "tls-output-cert-chain")]
    pub tls_output_cert_chain: Option<PathBuf>,
    /// path to custom TLS client key to use (PEM or DER PKCS#1, PKCS#8, or SEC1)
    #[arg(long = "tls-output-client-key")]
    pub tls_output_client_key: Option<PathBuf>,
    /// path to custom Certificate Authority certificates to use instead of web root CAs (PEM or DER format)
    #[arg(long = "tls-output-root-ca")]
    pub tls_output_root_ca: Option<PathBuf>,
    /// DANGER! Do not validate server identity.  Use with caution. DANGER!
    #[arg(long = "tls-output-skip-server-verify", default_value_t = false)]
    pub tls_output_skip_server_verify: bool,
}

/// Additional parameters needed for Iroh output (`--iroh-output`)
///
/// # Examples
///
/// ```
/// use datapipe::args::IrohOutputArgs;
///
/// let args = IrohOutputArgs::default();
/// assert!(args.is_empty());
/// ```
#[derive(Args, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct IrohOutputArgs {
    /// comma-separated direct IP socket addresses (`ip:port`) for the target Iroh endpoint
    #[arg(long = "iroh-output-addrs")]
    pub iroh_output_addrs: Option<String>,
    /// custom relay server URL (`http://` or `https://`) for reaching the target Iroh endpoint
    #[arg(long = "iroh-output-relay-url")]
    pub iroh_output_relay_url: Option<String>,
    /// custom ALPN protocol identifier (defaults to `datapipe/iroh/1`)
    #[arg(long = "iroh-output-alpn")]
    pub iroh_output_alpn: Option<String>,
    /// local 32-byte Ed25519 secret key in 64-character hex (or z-base-32) format
    #[arg(long = "iroh-output-secret-key")]
    pub iroh_output_secret_key: Option<String>,
    /// disable default relay servers (direct IP transport only)
    #[arg(long = "iroh-output-no-relay", default_value_t = false)]
    pub iroh_output_no_relay: bool,
}

impl std::fmt::Debug for IrohOutputArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IrohOutputArgs")
            .field("iroh_output_addrs", &self.iroh_output_addrs)
            .field("iroh_output_relay_url", &self.iroh_output_relay_url)
            .field("iroh_output_alpn", &self.iroh_output_alpn)
            .field(
                "iroh_output_secret_key",
                &self.iroh_output_secret_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("iroh_output_no_relay", &self.iroh_output_no_relay)
            .finish()
    }
}

/// Logging parameters
#[derive(Args, Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[group(required = false, multiple = true)]
#[serde(default)]
pub struct LoggingArgs {
    /// should logs be kept after the program exits?  defaults to false
    #[arg(long = "keep-logs", default_value_t = false)]
    pub keep_logs: bool,
    /// where should logs be written?  defaults to /var/tmp
    #[arg(long = "log-dir")]
    pub log_dir: Option<String>,
}

impl InputArgs {
    /// Returns true if no input source fields are configured
    pub fn is_empty(&self) -> bool {
        self.file_input.is_none()
            && self.http_input.is_none()
            && self.https_input.is_none()
            && !self.stdin_input
            && self.tcp_input.is_none()
            && self.tcp_listen_input.is_none()
            && self.tls_input.is_none()
            && self.tls_listen_input.is_none()
            && self.udp_input.is_none()
            && self.udp_multicast_input.is_none()
            && self.iroh_input.is_none()
            && !self.iroh_listen_input
    }
}

impl IrohInputArgs {
    /// Returns true if no Iroh input fields are configured
    pub fn is_empty(&self) -> bool {
        self.iroh_input_addrs.is_none()
            && self.iroh_input_relay_url.is_none()
            && self.iroh_input_alpn.is_none()
            && self.iroh_input_secret_key.is_none()
            && !self.iroh_input_no_relay
    }
}

impl IrohListenInputArgs {
    /// Returns true if no Iroh listen input fields are configured
    pub fn is_empty(&self) -> bool {
        self.iroh_listen_input_bind_addr.is_none()
            && self.iroh_listen_input_secret_key.is_none()
            && self.iroh_listen_input_alpn.is_none()
            && self.iroh_listen_input_relay_url.is_none()
            && self.iroh_listen_input_allowed_peer.is_none()
            && !self.iroh_listen_input_no_relay
    }
}

impl HttpInputArgs {
    /// Returns true if no HTTP input fields are configured
    pub fn is_empty(&self) -> bool {
        self.http_input_rate.is_none()
    }
}

impl HttpsInputArgs {
    /// Returns true if no HTTPS input fields are configured
    pub fn is_empty(&self) -> bool {
        self.https_input_rate.is_none()
            && self.https_input_root_certificates.is_none()
            && self.https_input_certificate_revocation_list.is_none()
            && self.https_input_client_identity.is_none()
            && !self.https_input_allow_invalid_hostnames
            && !self.https_input_allow_invalid_certificates
    }
}

impl TlsInputArgs {
    /// Returns true if no TLS input fields are configured
    pub fn is_empty(&self) -> bool {
        self.tls_input_cert_chain.is_none()
            && self.tls_input_client_key.is_none()
            && self.tls_input_root_ca.is_none()
            && !self.tls_input_skip_server_verify
    }
}

impl TlsListenInputArgs {
    /// Returns true if no TLS listen input fields are configured
    pub fn is_empty(&self) -> bool {
        self.tls_listen_input_cert_chain.is_none()
            && self.tls_listen_input_server_key.is_none()
            && !self.tls_listen_input_skip_client_verify
            && !self.tls_listen_input_generate_self_signed
    }
}

impl DecryptionArgs {
    /// Returns true if no decryption fields are configured
    pub fn is_empty(&self) -> bool {
        self.decryption_key.is_none() && !self.decrypt_allow_legacy
    }
}

impl EncryptionArgs {
    /// Returns true if no encryption fields are configured
    pub fn is_empty(&self) -> bool {
        self.encryption_key.is_none() && !self.generate_encryption_key
    }
}

impl OutputArgs {
    /// Returns true if no output destination fields are configured
    pub fn is_empty(&self) -> bool {
        self.file_output.is_none()
            && self.http_output.is_none()
            && self.https_output.is_none()
            && !self.stdout_output
            && self.tcp_output.is_none()
            && self.tls_output.is_none()
            && self.udp_output.is_none()
            && self.iroh_output.is_none()
    }
}

impl IrohOutputArgs {
    /// Returns true if no Iroh output fields are configured
    pub fn is_empty(&self) -> bool {
        self.iroh_output_addrs.is_none()
            && self.iroh_output_relay_url.is_none()
            && self.iroh_output_alpn.is_none()
            && self.iroh_output_secret_key.is_none()
            && !self.iroh_output_no_relay
    }
}

impl HttpOutputArgs {
    /// Returns true if no HTTP output fields are configured
    pub fn is_empty(&self) -> bool {
        self.http_output_rate.is_none()
            && self.http_output_delimiter.is_none()
            && self.http_output_include_delimiter
    }
}

impl HttpsOutputArgs {
    /// Returns true if no HTTPS output fields are configured
    pub fn is_empty(&self) -> bool {
        self.https_output_rate.is_none()
            && self.https_output_delimiter.is_none()
            && self.https_output_include_delimiter.is_none()
            && self.https_output_root_certificates.is_none()
            && self.https_output_certificate_revocation_list.is_none()
            && self.https_output_client_identity.is_none()
            && !self.https_output_allow_invalid_hostnames
            && !self.https_output_allow_invalid_certificates
    }
}

impl TlsOutputArgs {
    /// Returns true if no TLS output fields are configured
    pub fn is_empty(&self) -> bool {
        self.tls_output_cert_chain.is_none()
            && self.tls_output_client_key.is_none()
            && self.tls_output_root_ca.is_none()
            && !self.tls_output_skip_server_verify
    }
}

impl LoggingArgs {
    /// Returns true if default logging fields are configured
    pub fn is_empty(&self) -> bool {
        !self.keep_logs && self.log_dir.is_none()
    }
}

/// The embedded GNU Affero General Public License (AGPL-3.0-only) for `datapipe`.
pub const LICENSE: &str = include_str!("../LICENSE");

/// Returns the name, version, and copyright information for `datapipe`.
///
/// # Examples
///
/// ```
/// use datapipe::args::version_info;
///
/// let info = version_info();
/// assert!(info.contains("datapipe"));
/// assert!(info.contains(env!("CARGO_PKG_VERSION")));
/// assert!(info.contains("Copyright"));
/// ```
#[must_use]
pub fn version_info() -> String {
    format!(
        "{} {}\nCopyright (C) 2024-2026 {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_AUTHORS")
    )
}

/// Returns the embedded license text for `datapipe`.
///
/// # Examples
///
/// ```
/// use datapipe::args::license_info;
///
/// let lic = license_info();
/// assert!(lic.contains("GNU AFFERO GENERAL PUBLIC LICENSE"));
/// ```
#[must_use]
pub fn license_info() -> &'static str {
    LICENSE
}

/// Overall command line args
#[derive(Parser, Debug, Clone, PartialEq, Default)]
pub struct ProgramArgs {
    /// Print version, name, and copyright information and exit
    #[arg(long = "version", short = 'V')]
    pub version: bool,

    /// Print license information and exit
    #[arg(long = "license")]
    pub license: bool,

    /// Start datapipe using a configuration file
    #[arg(long = "use-config", group = "config_action")]
    pub use_config: Option<PathBuf>,

    /// Save the given datapipe command line parameters to a configuration file
    #[arg(long = "save-to-config", group = "config_action")]
    pub save_to_config: Option<PathBuf>,

    /// Check that the provided configuration file is a valid datapipe configuration file
    #[arg(long = "verify-config", group = "config_action")]
    pub verify_config: Option<PathBuf>,

    /// Disable metrics tracking and live display
    #[arg(long = "no-metrics", default_value_t = false)]
    pub no_metrics: bool,

    #[command(flatten)]
    pub input: InputArgs,
    #[command(flatten)]
    pub http_input: HttpInputArgs,
    #[command(flatten)]
    pub https_input: HttpsInputArgs,
    #[command(flatten)]
    pub tls_input: TlsInputArgs,
    #[command(flatten)]
    pub tls_listen_input: TlsListenInputArgs,
    #[command(flatten)]
    pub iroh_input: IrohInputArgs,
    #[command(flatten)]
    pub iroh_listen_input: IrohListenInputArgs,
    #[command(flatten)]
    pub decryption_args: DecryptionArgs,
    #[command(flatten)]
    pub encryption_args: EncryptionArgs,
    #[command(flatten)]
    pub output: OutputArgs,
    #[command(flatten)]
    pub http_output: HttpOutputArgs,
    #[command(flatten)]
    pub https_output: HttpsOutputArgs,
    #[command(flatten)]
    pub tls_output: TlsOutputArgs,
    #[command(flatten)]
    pub iroh_output: IrohOutputArgs,
    #[command(flatten)]
    pub logging_args: LoggingArgs,
}

impl ProgramArgs {
    /// Returns true if any command-line input or output arguments were provided
    pub fn has_cli_io(&self) -> bool {
        !self.input.is_empty() || !self.output.is_empty()
    }

    /// Validate the program arguments according to CLI requirements
    pub fn validate(&self) -> Result<(), DatapipeError> {
        if self.version || self.license {
            return Ok(());
        }

        if self.verify_config.is_some() {
            if self.has_cli_io() {
                let error_message =
                    "--verify-config cannot be combined with command-line input/output arguments"
                        .to_string();
                error!("{error_message}");
                return Err(DatapipeError::ValidationError(error_message));
            }
            return Ok(());
        }

        if self.use_config.is_some() {
            if self.has_cli_io() {
                let error_message =
                    "--use-config cannot be combined with command-line input/output arguments"
                        .to_string();
                error!("{error_message}");
                return Err(DatapipeError::ValidationError(error_message));
            }
            return Ok(());
        }

        if self.input.is_empty() {
            let error_message =
                "No input source provided! Please configure an input source or use --use-config."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        if self.decryption_args.decrypt_allow_legacy
            && self.decryption_args.decryption_key.is_none()
        {
            let error_message =
                "--decrypt-allow-legacy requires --decrypt to also be specified".to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        if self.output.is_empty() {
            let error_message =
                "No output destination provided! Please configure an output destination."
                    .to_string();
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }

        Ok(())
    }

    /// Ensure that the input reader is set only once
    fn check_reader_set(maybe_reader: &Option<Reader>) -> Result<(), DatapipeError> {
        match maybe_reader.as_ref() {
            Some(reader) => {
                let error_message = format!(
                    "Input previously assigned as {:?}; only one input can be used.",
                    reader
                );
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
            None => Ok(()),
        }
    }

    /// Prepare a reader for file input
    async fn handle_file_input(&self) -> Result<Reader, DatapipeError> {
        let file_path = self.input.file_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --file-input path".to_string())
        })?;
        let file_reader = FileReader::new(file_path).await?;
        info!("Using FILE input");
        Ok(Reader::File(file_reader))
    }

    /// Prepare a reader for HTTP input
    fn handle_http_input(&self) -> Result<Reader, DatapipeError> {
        let url = self.input.http_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --http-input URL".to_string())
        })?;
        let update_rate = match self.http_input.http_input_rate {
            Some(rate) => {
                info!("Using HTTP input rate of {} milliseconds", rate);
                rate
            }
            None => {
                let default_rate = HttpReader::DEFAULT_UPDATE_RATE;
                info!(
                    "Using default HTTP input rate of {} milliseconds",
                    default_rate
                );
                default_rate
            }
        };
        let http_reader = HttpReader::new(url, update_rate)?;
        info!("Using HTTP input");
        Ok(Reader::Http(http_reader))
    }

    /// Prepare a reader for HTTPS input
    async fn handle_https_input(&self) -> Result<Reader, DatapipeError> {
        let url = self.input.https_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --https-input URL".to_string())
        })?;

        let allow_invalid_hostnames = self.https_input.https_input_allow_invalid_hostnames;
        let allow_invalid_certs = self.https_input.https_input_allow_invalid_certificates;

        let read_rate = match self.https_input.https_input_rate {
            Some(rate_millis) => Duration::from_millis(rate_millis),
            None => HttpsReader::DEFAULT_READ_RATE,
        };

        let (maybe_root_certs, maybe_crls, maybe_identity) = load_https_tls_assets(
            "HTTPS input",
            self.https_input.https_input_root_certificates.as_deref(),
            self.https_input
                .https_input_certificate_revocation_list
                .as_deref(),
            self.https_input.https_input_client_identity.as_deref(),
        )
        .await?;

        let https_reader = HttpsReader::new(
            url,
            read_rate,
            maybe_root_certs,
            maybe_crls,
            maybe_identity,
            allow_invalid_hostnames,
            allow_invalid_certs,
        )?;
        info!("Using HTTPS input");
        Ok(Reader::Https(https_reader))
    }

    /// Prepare a reader for STDIN input
    fn handle_stdin_input(&self) -> Reader {
        info!("Using STDIN input");
        Reader::Stdin(StdinReader::new())
    }

    async fn handle_tcp_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.tcp_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --tcp-input address".to_string())
        })?;
        match TcpReaderWriter::new(address).await {
            Ok(tcp_reader) => Ok(Reader::Tcp(tcp_reader)),
            Err(error) => {
                let error_message =
                    format!("TCP input error {}: {}", address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    async fn handle_tcp_listen_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.tcp_listen_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --tcp-listen-input address".to_string())
        })?;
        match TcpListenReader::new(address).await {
            Ok(tcp_listen_reader) => Ok(Reader::TcpListen(tcp_listen_reader)),
            Err(error) => {
                let error_message = format!(
                    "TCP listen input error {}: {}",
                    address,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    async fn handle_tls_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.tls_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --tls-input address".to_string())
        })?;
        match self.get_tls_input_config() {
            Ok(tls_config) => match TlsReaderWriter::new(address, tls_config).await {
                Ok(tls_reader) => {
                    info!("Using TLS input");
                    Ok(Reader::Tls(tls_reader))
                }
                Err(error) => {
                    let error_message =
                        format!("TLS input error {}: {}", address, error_root_cause(&error));
                    error!("{error_message}");
                    Err(DatapipeError::InputOutputError(error_message))
                }
            },
            Err(error) => {
                let error_message = format!("TLS input setup error: {}", error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    fn setup_root_cert_store(&self) -> Result<RootCertStore, DatapipeError> {
        build_tls_root_cert_store(self.tls_input.tls_input_root_ca.as_ref(), "TLS input")
    }

    fn get_tls_input_client_config_builder(
        &self,
    ) -> Result<ConfigBuilder<ClientConfig, WantsClientCert>, DatapipeError> {
        // check if no verification was requested
        if self.tls_input.tls_input_skip_server_verify {
            let dangerous_config = ConfigBuilder::dangerous(ClientConfig::builder());
            Ok(dangerous_config
                .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new())))
        } else {
            let root_cert_store = self.setup_root_cert_store()?;
            Ok(ClientConfig::builder().with_root_certificates(root_cert_store))
        }
    }

    // TLS input-specific wrapper for get_tls_cert_chain
    fn get_tls_input_certificate_chain(
        &self,
    ) -> Result<Option<Vec<CertificateDer<'static>>>, DatapipeError> {
        match self.tls_input.tls_input_cert_chain.as_ref() {
            Some(tls_cert_chain_path) => {
                // get certificate chain
                match get_tls_cert_chain(tls_cert_chain_path) {
                    Ok(cert_chain) => {
                        info!("Success getting TLS input certificate chain");
                        Ok(Some(cert_chain))
                    }
                    Err(error) => {
                        // failed to get cert chain
                        let error_message = format!(
                            "Error getting TLS input certificate chain {:?}: {}",
                            tls_cert_chain_path,
                            error_root_cause(&error)
                        );
                        error!("{error_message}");
                        Err(DatapipeError::InputOutputError(error_message))
                    }
                }
            }
            None => {
                // make sure the user did not give a client certificate too
                if self.tls_input.tls_input_client_key.is_some() {
                    // error: a cert chain is needed if the user is providing a client key
                    let error_message = "TLS input client key (--tls-input-client-key) requires certificate chain (--tls-input-cert-chain) to also be used";
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message.to_string()));
                }
                info!("No TLS certificate chain provided");
                Ok(None)
            }
        }
    }

    // TLS input-specific wrapper for get_tls_private_key
    fn get_tls_input_client_key(&self) -> Result<Option<PrivateKeyDer<'static>>, DatapipeError> {
        match self.tls_input.tls_input_client_key.as_ref() {
            Some(tls_client_key_path) => {
                match get_tls_private_key(tls_client_key_path) {
                    Ok(client_key) => {
                        info!("Success getting TLS input client key");
                        Ok(Some(client_key))
                    }
                    Err(error) => {
                        // failed to get client key
                        let error_message = format!(
                            "Error getting TLS input client key {:?}: {}",
                            tls_client_key_path,
                            error_root_cause(&error)
                        );
                        error!("{error_message}");
                        Err(DatapipeError::InputOutputError(error_message))
                    }
                }
            }
            None => {
                // make sure the user did not give a certificate chain too
                if self.tls_input.tls_input_cert_chain.is_some() {
                    let error_message = "TLS input certificate chain (--tls-input-cert-chain) requires client key (--tls-input-client-key) to also be used";
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message.to_string()));
                }
                info!("No TLS input client key provided");
                Ok(None)
            }
        }
    }

    fn get_tls_input_config(&self) -> Result<ClientConfig, DatapipeError> {
        let config_builder = self.get_tls_input_client_config_builder()?;
        let maybe_cert_chain = self.get_tls_input_certificate_chain()?;
        let maybe_client_key = self.get_tls_input_client_key()?;
        // both get_tls_input_certificate_chain and get_tls_input_client_key
        // check to make sure if one is present the other is also present
        if let Some(cert_chain) = maybe_cert_chain
            && let Some(client_key) = maybe_client_key
        {
            // finish building the config with cert chain and client key
            match config_builder.with_client_auth_cert(cert_chain, client_key) {
                Ok(tls_config) => Ok(tls_config),
                Err(error) => {
                    let error_message = format!(
                        "Error creating TLS input config with cert chain and client key: {}",
                        error_root_cause(&error)
                    );
                    error!("{error_message}");
                    Err(DatapipeError::InputOutputError(error_message))
                }
            }
        } else {
            // finishing build the config with no client authentication
            Ok(config_builder.with_no_client_auth())
        }
    }

    // TLS listen input-specific wrapper for get_tls_cert_chain
    fn get_tls_listen_input_certificate_chain(
        &self,
    ) -> Result<Option<Vec<CertificateDer<'static>>>, DatapipeError> {
        match self.tls_listen_input.tls_listen_input_cert_chain.as_ref() {
            Some(tls_cert_chain_path) => {
                // get certificate chain
                match get_tls_cert_chain(tls_cert_chain_path) {
                    Ok(cert_chain) => {
                        info!("Success getting TLS listen input certificate chain");
                        Ok(Some(cert_chain))
                    }
                    Err(error) => {
                        // failed to get cert chain
                        let error_message = format!(
                            "Error getting TLS listen input certificate chain {:?}: {}",
                            tls_cert_chain_path,
                            error_root_cause(&error)
                        );
                        error!("{error_message}");
                        Err(DatapipeError::InputOutputError(error_message))
                    }
                }
            }
            None => {
                // TLS listen input requires a server cert chain and private key
                // if a cert chain is not provided, and it is not being generated, it is an error
                if !self.tls_listen_input.tls_listen_input_generate_self_signed {
                    let error_message = "TLS listen input requires certificate chain to be provided (--tls-listen-input-cert-chain) or generated (--tls-listen-input-generate-self-signed)";
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message.to_string()));
                }
                info!("No TLS certificate chain provided");
                Ok(None)
            }
        }
    }

    // TLS input-specific wrapper for get_tls_private_key
    fn get_tls_listen_input_server_key(
        &self,
    ) -> Result<Option<PrivateKeyDer<'static>>, DatapipeError> {
        match self.tls_listen_input.tls_listen_input_server_key.as_ref() {
            Some(tls_server_key_path) => {
                match get_tls_private_key(tls_server_key_path) {
                    Ok(server_key) => {
                        info!("Success getting TLS listen input server key");
                        Ok(Some(server_key))
                    }
                    Err(error) => {
                        // failed to get server key
                        let error_message = format!(
                            "Error getting TLS listen input server key {:?}: {}",
                            tls_server_key_path,
                            error_root_cause(&error)
                        );
                        error!("{error_message}");
                        Err(DatapipeError::InputOutputError(error_message))
                    }
                }
            }
            None => {
                // TLS listen input requires a server cert chain and private key
                // if a server private key is not provided, and it is not being generated, it is an error
                if !self.tls_listen_input.tls_listen_input_generate_self_signed {
                    let error_message = "TLS listen input requires server private key to be provided (--tls-listen-input-server-key) or generated (--tls-listen-input-generate-self-signed)";
                    error!("{error_message}");
                    return Err(DatapipeError::ConfigurationError(error_message.to_string()));
                }
                info!("No TLS listen input server key provided");
                Ok(None)
            }
        }
    }

    fn generate_self_signed(&self) -> Result<CertifiedKey<KeyPair>, DatapipeError> {
        let hostname = crate::utilities::hostname();
        let subject_alt_names = vec![hostname, "localhost".to_string()];
        match generate_simple_self_signed(subject_alt_names) {
            Ok(certified_key) => Ok(certified_key),
            Err(error) => {
                let error_message = format!("Error generating self-signed certificate: {error}");
                error!("{error_message}");
                Err(DatapipeError::ConfigurationError(error_message))
            }
        }
    }

    fn get_tls_listen_input_config(&self) -> Result<ServerConfig, DatapipeError> {
        let maybe_cert_chain = self.get_tls_listen_input_certificate_chain()?;
        let maybe_server_key = self.get_tls_listen_input_server_key()?;
        let (cert_chain, server_key) = match (maybe_cert_chain, maybe_server_key) {
            (None, None) => {
                // generate a self-signed cert and keys
                let CertifiedKey { cert, signing_key } = self.generate_self_signed()?;
                (vec![cert.der().clone()], PrivateKeyDer::from(signing_key))
            }
            (Some(cert_chain), Some(server_key)) => (cert_chain, server_key),
            _ => {
                let error_message = "Both certificate chain and server key must be provided together for TLS listen input";
                error!("{error_message}");
                return Err(DatapipeError::ConfigurationError(error_message.to_string()));
            }
        };
        let server_config = if self.tls_listen_input.tls_listen_input_skip_client_verify {
            ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(cert_chain, server_key)?
        } else {
            let mut roots = RootCertStore::empty();
            let (certs_added_count, _certs_ignored_count) =
                roots.add_parsable_certificates(cert_chain.clone());
            if certs_added_count == 0 {
                let error_message = "No valid certificates could be added to TLS listen client verification root store".to_string();
                error!("{error_message}");
                return Err(DatapipeError::ValidationError(error_message));
            }
            let client_cert_verifier = WebPkiClientVerifier::builder(roots.into()).build()?;
            ServerConfig::builder()
                .with_client_cert_verifier(client_cert_verifier)
                .with_single_cert(cert_chain, server_key)?
        };
        Ok(server_config)
    }

    /// Prepare a reader for TLS listen input
    async fn handle_tls_listen_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.tls_listen_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --tls-listen-input address".to_string())
        })?;
        match self.get_tls_listen_input_config() {
            Ok(tls_config) => match TlsListenReader::new(address, tls_config).await {
                Ok(tls_listen_reader) => {
                    info!("Using TLS listen input");
                    Ok(Reader::TlsListen(tls_listen_reader))
                }
                Err(error) => {
                    let error_message = format!(
                        "TLS listen input error {}: {}",
                        address,
                        error_root_cause(&error)
                    );
                    error!("{error_message}");
                    Err(DatapipeError::InputOutputError(error_message))
                }
            },
            Err(error) => {
                let error_message =
                    format!("TLS listen input setup error: {}", error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    /// Prepare a reader for UDP input
    async fn handle_udp_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.udp_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --udp-input address".to_string())
        })?;
        match UdpReader::new(address).await {
            Ok(udp_reader) => {
                info!("Using UDP input");
                Ok(Reader::Udp(udp_reader))
            }
            Err(error) => {
                let error_message = format!(
                    "Cannot open input UDP address {:?}: {}",
                    address,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    /// Prepare a reader for UDP multicast input
    async fn handle_udp_multicast_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.udp_multicast_input.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --udp-multicast-input address".to_string())
        })?;
        match UdpReader::new_multicast(address).await {
            Ok(udp_reader) => {
                info!("Using UDP multicast input");
                Ok(Reader::Udp(udp_reader))
            }
            Err(error) => {
                let error_message = format!(
                    "Cannot open input UDP multicast address {:?}: {}",
                    address,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    /// Prepare a reader for Iroh outbound input
    async fn handle_iroh_input(&self) -> Result<Reader, DatapipeError> {
        let Some(ref peer_str) = self.input.iroh_input else {
            return Err(DatapipeError::ValidationError(
                "Missing --iroh-input EndpointId".to_string(),
            ));
        };
        let peer_id = parse_iroh_endpoint_id(peer_str)?;
        let direct_addrs = match self.iroh_input.iroh_input_addrs {
            Some(ref addrs_str) => parse_iroh_socket_addrs(addrs_str)?,
            None => Vec::new(),
        };
        let relay_url = match self.iroh_input.iroh_input_relay_url {
            Some(ref url_str) => Some(parse_iroh_relay_url(url_str)?),
            None => None,
        };
        let alpn = match self.iroh_input.iroh_input_alpn {
            Some(ref alpn_str) => {
                if alpn_str.trim().is_empty() {
                    let error_message = "Iroh input ALPN cannot be empty".to_string();
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message));
                }
                alpn_str.as_bytes().to_vec()
            }
            None => DEFAULT_IROH_ALPN.to_vec(),
        };
        let secret_key = match self.iroh_input.iroh_input_secret_key {
            Some(ref key_str) => Some(parse_iroh_secret_key(key_str)?),
            None => None,
        };
        let config = IrohConnectConfig {
            target_id: peer_id,
            target_addrs: direct_addrs,
            relay_url,
            alpn,
            secret_key,
            no_relay: self.iroh_input.iroh_input_no_relay,
        };
        match IrohReader::new(config).await {
            Ok(iroh_reader) => {
                info!("Using Iroh input connected to peer {}", peer_id);
                Ok(Reader::Iroh(iroh_reader))
            }
            Err(error) => {
                let error_message = format!(
                    "Iroh input error connecting to {}: {}",
                    peer_id,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    /// Prepare a reader for Iroh listen input
    async fn handle_iroh_listen_input(&self) -> Result<Reader, DatapipeError> {
        let bind_addr = match self.iroh_listen_input.iroh_listen_input_bind_addr {
            Some(ref addr_str) => {
                let parsed = addr_str
                    .trim()
                    .parse::<std::net::SocketAddr>()
                    .map_err(|err| {
                        let error_message =
                            format!("Invalid Iroh listen bind address '{addr_str}': {err}");
                        error!("{error_message}");
                        DatapipeError::ValidationError(error_message)
                    })?;
                Some(parsed)
            }
            None => None,
        };
        let secret_key = match self.iroh_listen_input.iroh_listen_input_secret_key {
            Some(ref key_str) => Some(parse_iroh_secret_key(key_str)?),
            None => None,
        };
        let alpn = match self.iroh_listen_input.iroh_listen_input_alpn {
            Some(ref alpn_str) => {
                if alpn_str.trim().is_empty() {
                    let error_message = "Iroh listen input ALPN cannot be empty".to_string();
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message));
                }
                alpn_str.as_bytes().to_vec()
            }
            None => DEFAULT_IROH_ALPN.to_vec(),
        };
        let relay_url = match self.iroh_listen_input.iroh_listen_input_relay_url {
            Some(ref url_str) => Some(parse_iroh_relay_url(url_str)?),
            None => None,
        };
        let allowed_peer = match self.iroh_listen_input.iroh_listen_input_allowed_peer {
            Some(ref peer_str) => Some(parse_iroh_endpoint_id(peer_str)?),
            None => None,
        };
        let config = IrohListenConfig {
            bind_addr,
            secret_key,
            alpn,
            relay_url,
            allowed_peer,
            no_relay: self.iroh_listen_input.iroh_listen_input_no_relay,
        };
        match IrohListenReader::new(config).await {
            Ok(iroh_listen_reader) => {
                info!(
                    "Using Iroh listen input on EndpointId: {} (local addrs: {:?})",
                    iroh_listen_reader.endpoint_id(),
                    iroh_listen_reader.local_addrs()
                );
                Ok(Reader::IrohListen(iroh_listen_reader))
            }
            Err(error) => {
                let error_message =
                    format!("Iroh listen input error: {}", error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    /// Select the wanted input implementation from the command line args
    pub(crate) async fn get_input_reader(&self) -> Result<Reader, DatapipeError> {
        let mut maybe_reader: Option<Reader> = None;
        if self.input.file_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_file_input().await?);
        }
        if self.input.http_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_http_input()?);
        }
        if self.input.https_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_https_input().await?);
        }
        if self.input.stdin_input {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_stdin_input());
        }
        if self.input.tcp_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_tcp_input().await?);
        }
        if self.input.tcp_listen_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_tcp_listen_input().await?);
        }
        if self.input.tls_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_tls_input().await?);
        }
        if self.input.tls_listen_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_tls_listen_input().await?);
        }
        if self.input.udp_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_udp_input().await?);
        }
        if self.input.udp_multicast_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_udp_multicast_input().await?);
        }
        if self.input.iroh_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_iroh_input().await?);
        }
        if self.input.iroh_listen_input {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_iroh_listen_input().await?);
        }
        match maybe_reader {
            Some(reader) => Ok(reader),
            None => {
                let error_message = "No input source provided!";
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message.to_string()))
            }
        }
    }

    async fn handle_file_output(&self) -> Result<Writer, DatapipeError> {
        let file_path = self.output.file_output.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --file-output path".to_string())
        })?;
        match FileWriter::new(file_path).await {
            Ok(file_writer) => {
                info!("Using FILE output for path: {:?}", file_path);
                Ok(Writer::File(file_writer))
            }
            Err(error) => {
                let error_message = format!(
                    "File output error {:?}: {}",
                    file_path,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    fn handle_http_output(&self) -> Result<Writer, DatapipeError> {
        let url = self.output.http_output.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --http-output URL".to_string())
        })?;
        let delimiter: Vec<u8> = match self.http_output.http_output_delimiter {
            Some(ref d) => d.clone(),
            None => HttpWriter::DEFAULT_DELIMITER.to_vec(),
        };
        let include_delimiter = self.http_output.http_output_include_delimiter;
        let output_rate: Duration = match self.http_output.http_output_rate {
            Some(rate_millis) => Duration::from_millis(rate_millis),
            None => HttpWriter::DEFAULT_WRITE_RATE,
        };
        match HttpWriter::new(url, delimiter, include_delimiter, output_rate) {
            Ok(http_writer) => {
                info!("Using HTTP output");
                Ok(Writer::Http(http_writer))
            }
            Err(error) => {
                let error_message = format!("HTTP URL error {}: {}", url, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    async fn handle_https_output(&self) -> Result<Writer, DatapipeError> {
        let url = self.output.https_output.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --https-output URL".to_string())
        })?;

        let allow_invalid_hostnames = self.https_output.https_output_allow_invalid_hostnames;
        let allow_invalid_certs = self.https_output.https_output_allow_invalid_certificates;

        let write_rate = match self.https_output.https_output_rate {
            Some(rate_millis) => Duration::from_millis(rate_millis),
            None => HttpsWriter::DEFAULT_WRITE_RATE,
        };

        let delimiter: Vec<u8> = match self.https_output.https_output_delimiter {
            Some(ref d) => d.clone(),
            None => HttpsWriter::DEFAULT_DELIMITER.to_vec(),
        };

        let include_delimiter = self
            .https_output
            .https_output_include_delimiter
            .unwrap_or(true);

        let (maybe_root_certs, maybe_crls, maybe_identity) = load_https_tls_assets(
            "HTTPS output",
            self.https_output.https_output_root_certificates.as_deref(),
            self.https_output
                .https_output_certificate_revocation_list
                .as_deref(),
            self.https_output.https_output_client_identity.as_deref(),
        )
        .await?;

        match HttpsWriter::new(
            url,
            delimiter,
            include_delimiter,
            write_rate,
            maybe_root_certs,
            maybe_crls,
            maybe_identity,
            allow_invalid_hostnames,
            allow_invalid_certs,
        ) {
            Ok(https_writer) => {
                info!("Using HTTPS output");
                Ok(Writer::Https(https_writer))
            }
            Err(error) => {
                let error_message = format!(
                    "HTTPS output URL error {}: {}",
                    url,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    fn handle_stdout_writer(&self) -> Writer {
        info!("Using STDOUT output");
        Writer::Stdout(StdoutWriter::new())
    }

    async fn handle_tcp_output(&self) -> Result<Writer, DatapipeError> {
        let address = self.output.tcp_output.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --tcp-output address".to_string())
        })?;
        match TcpReaderWriter::new(address).await {
            Ok(tcp_writer) => Ok(Writer::Tcp(tcp_writer)),
            Err(error) => {
                let error_message =
                    format!("TCP output error {}: {}", address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    async fn handle_tls_output(&self) -> Result<Writer, DatapipeError> {
        let address = self.output.tls_output.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --tls-output address".to_string())
        })?;
        let tls_config = self.get_tls_output_config()?;
        match TlsReaderWriter::new(address, tls_config).await {
            Ok(tls_writer) => {
                info!("Using TLS output");
                Ok(Writer::Tls(Box::new(tls_writer)))
            }
            Err(error) => {
                let error_message =
                    format!("TLS output error {}: {}", address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    fn get_tls_output_config(&self) -> Result<ClientConfig, DatapipeError> {
        let root_cert_store =
            build_tls_root_cert_store(self.tls_output.tls_output_root_ca.as_ref(), "TLS output")?;
        // begin build the config
        // check if no verification was requested
        let config: ConfigBuilder<ClientConfig, WantsClientCert> =
            if self.tls_output.tls_output_skip_server_verify {
                let dangerous_config = ConfigBuilder::dangerous(ClientConfig::builder());
                dangerous_config
                    .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new()))
            } else {
                ClientConfig::builder().with_root_certificates(root_cert_store)
            };
        // see if client auth is needed
        match self.tls_output.tls_output_cert_chain.as_ref() {
            Some(tls_cert_chain_path) => {
                // get certificate chain
                let cert_chain = get_tls_cert_chain(tls_cert_chain_path)?;
                // get client certificate
                match self.tls_output.tls_output_client_key.as_ref() {
                    Some(tls_client_key_path) => {
                        let client_key = get_tls_private_key(tls_client_key_path)?;
                        // finish building the config with cert chain and client key
                        match config.with_client_auth_cert(cert_chain, client_key) {
                            Ok(tls_config) => Ok(tls_config),
                            Err(error) => {
                                let error_message = format!(
                                    "Error creating TLS output config with cert chain and client key: {}",
                                    error_root_cause(&error)
                                );
                                error!("{error_message}");
                                Err(DatapipeError::ValidationError(error_message))
                            }
                        }
                    }
                    None => {
                        // the user should have provided a client key too
                        let error_message = "TLS output certificate chain (--tls-output-cert-chain) requires client key (--tls-output-client-key) to also be used";
                        error!("{error_message}");
                        Err(DatapipeError::ValidationError(error_message.to_string()))
                    }
                }
            }
            None => {
                // make sure the user did not give a client certificate too
                if self.tls_output.tls_output_client_key.is_some() {
                    let error_message = "TLS output client key (--tls-output-client-key) requires certificate chain (--tls-output-cert-chain) to also be used";
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message.to_string()));
                }
                // finishing build the config with no client authentication
                Ok(config.with_no_client_auth())
            }
        }
    }

    async fn handle_udp_output(&self) -> Result<Writer, DatapipeError> {
        let address = self.output.udp_output.as_ref().ok_or_else(|| {
            DatapipeError::ValidationError("Missing --udp-output address".to_string())
        })?;
        match UdpWriter::new(address).await {
            Ok(udp_writer) => {
                info!("Using UDP output");
                Ok(Writer::Udp(udp_writer))
            }
            Err(error) => {
                let error_message =
                    format!("UDP output error {}: {}", address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    async fn handle_iroh_output(&self) -> Result<Writer, DatapipeError> {
        let Some(ref peer_str) = self.output.iroh_output else {
            return Err(DatapipeError::ValidationError(
                "Missing --iroh-output EndpointId".to_string(),
            ));
        };
        let peer_id = parse_iroh_endpoint_id(peer_str)?;
        let direct_addrs = match self.iroh_output.iroh_output_addrs {
            Some(ref addrs_str) => parse_iroh_socket_addrs(addrs_str)?,
            None => Vec::new(),
        };
        let relay_url = match self.iroh_output.iroh_output_relay_url {
            Some(ref url_str) => Some(parse_iroh_relay_url(url_str)?),
            None => None,
        };
        let alpn = match self.iroh_output.iroh_output_alpn {
            Some(ref alpn_str) => {
                if alpn_str.trim().is_empty() {
                    let error_message = "Iroh output ALPN cannot be empty".to_string();
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message));
                }
                alpn_str.as_bytes().to_vec()
            }
            None => DEFAULT_IROH_ALPN.to_vec(),
        };
        let secret_key = match self.iroh_output.iroh_output_secret_key {
            Some(ref key_str) => Some(parse_iroh_secret_key(key_str)?),
            None => None,
        };
        let config = IrohConnectConfig {
            target_id: peer_id,
            target_addrs: direct_addrs,
            relay_url,
            alpn,
            secret_key,
            no_relay: self.iroh_output.iroh_output_no_relay,
        };
        match IrohWriter::new(config).await {
            Ok(iroh_writer) => {
                info!("Using Iroh output connected to peer {}", peer_id);
                Ok(Writer::Iroh(iroh_writer))
            }
            Err(error) => {
                let error_message = format!(
                    "Iroh output error connecting to {}: {}",
                    peer_id,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    pub(crate) async fn get_output_writers(&self) -> Result<Vec<Writer>, DatapipeError> {
        let mut writers: Vec<Writer> = Vec::new();
        if self.output.file_output.is_some() {
            let file_writer = self.handle_file_output().await?;
            writers.push(file_writer);
        }
        if self.output.http_output.is_some() {
            let http_writer = self.handle_http_output()?;
            writers.push(http_writer);
        }
        if self.output.https_output.is_some() {
            let https_writer = self.handle_https_output().await?;
            writers.push(https_writer);
        }
        if self.output.stdout_output {
            let stdout_writer = self.handle_stdout_writer();
            writers.push(stdout_writer);
        }
        if self.output.tcp_output.is_some() {
            let tcp_writer = self.handle_tcp_output().await?;
            writers.push(tcp_writer);
        }
        if self.output.tls_output.is_some() {
            let tls_writer = self.handle_tls_output().await?;
            writers.push(tls_writer);
        }
        if self.output.udp_output.is_some() {
            let udp_writer = self.handle_udp_output().await?;
            writers.push(udp_writer);
        }
        if self.output.iroh_output.is_some() {
            let iroh_writer = self.handle_iroh_output().await?;
            writers.push(iroh_writer);
        }
        if writers.is_empty() {
            let error_message = "No output destination provided!";
            error!("{error_message}");
            Err(DatapipeError::ValidationError(error_message.to_string()))
        } else {
            Ok(writers)
        }
    }

    pub(crate) fn get_encryption_args(&self) -> Result<Option<StreamEncryptor>, DatapipeError> {
        if self.encryption_args.generate_encryption_key {
            let encryption_key = EncryptionKey::generate();
            eprintln!("Generated encryption key: {}", encryption_key);
            let encryptor = StreamEncryptor::new(encryption_key)?;
            return Ok(Some(encryptor));
        }
        if let Some(ref key_str) = self.encryption_args.encryption_key {
            let encryption_key = EncryptionKey::new(key_str)?;
            let encryptor = StreamEncryptor::new(encryption_key)?;
            return Ok(Some(encryptor));
        }
        Ok(None)
    }

    pub(crate) fn get_decryption_args(&self) -> Result<Option<StreamDecryptor>, DatapipeError> {
        if let Some(ref key_str) = self.decryption_args.decryption_key {
            let encryption_key = EncryptionKey::new(key_str)?;
            let decryptor = StreamDecryptor::new_with_legacy(
                encryption_key,
                self.decryption_args.decrypt_allow_legacy,
            )?;
            return Ok(Some(decryptor));
        }
        Ok(None)
    }

    pub async fn to_parameters(&self) -> Result<Parameters, DatapipeError> {
        let reader = self.get_input_reader().await?;
        let writers = self.get_output_writers().await?;
        let maybe_decryptor = self.get_decryption_args()?;
        let maybe_encryptor = self.get_encryption_args()?;
        let metrics = if self.no_metrics {
            None
        } else {
            Some(crate::metrics::DatapipeMetrics::new())
        };

        Ok(Parameters {
            reader,
            maybe_decryptor,
            maybe_encryptor,
            writers,
            metrics,
        })
    }
}

fn build_tls_root_cert_store(
    maybe_root_ca: Option<&PathBuf>,
    label: &str,
) -> Result<RootCertStore, DatapipeError> {
    let mut root_cert_store = RootCertStore::empty();
    if let Some(root_ca) = maybe_root_ca {
        get_root_ca(root_ca, &mut root_cert_store).map_err(|error| {
            let error_message = format!(
                "Error loading {label} certificate authority (CA) roots: {}",
                error_root_cause(&error)
            );
            error!("{error_message}");
            DatapipeError::InputOutputError(error_message)
        })?;
    } else {
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
    }
    Ok(root_cert_store)
}

fn contains_pem_marker(bytes: &[u8]) -> bool {
    bytes.windows(10).any(|w| w == b"-----BEGIN")
}

async fn load_https_tls_assets(
    label: &str,
    root_cert_path: Option<&Path>,
    crl_path: Option<&Path>,
    identity_path: Option<&Path>,
) -> Result<
    (
        Option<Vec<Certificate>>,
        Option<Vec<CertificateRevocationList>>,
        Option<Identity>,
    ),
    DatapipeError,
> {
    let maybe_root_certs = if let Some(path) = root_cert_path {
        let bytes = tokio::fs::read(path).await?;
        if bytes.is_empty() {
            let error_message = format!("{label} root certificate file {:?} is empty", path);
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        let root_certs = if contains_pem_marker(&bytes) {
            Certificate::from_pem_bundle(&bytes).map_err(|error| {
                let error_message = format!(
                    "Error getting {label} PEM root certificate at path {:?}: {}",
                    path,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                DatapipeError::ValidationError(error_message)
            })?
        } else {
            let cert = Certificate::from_der(&bytes).map_err(|error| {
                let error_message = format!(
                    "Error getting {label} DER root certificate at path {:?}: {}",
                    path,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                DatapipeError::ValidationError(error_message)
            })?;
            vec![cert]
        };
        if root_certs.is_empty() {
            let error_message = format!(
                "No certificates found in {label} root certificate file {:?}",
                path
            );
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        Some(root_certs)
    } else {
        None
    };

    let maybe_crls = if let Some(path) = crl_path {
        let bytes = tokio::fs::read(path).await?;
        if bytes.is_empty() {
            let error_message = format!("{label} CRL file {:?} is empty", path);
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        let crls = CertificateRevocationList::from_pem_bundle(&bytes).map_err(|error| {
            let error_message = format!(
                "Error getting {label} certificate revocation list at path {:?}: {}",
                path,
                error_root_cause(&error)
            );
            error!("{error_message}");
            DatapipeError::ValidationError(error_message)
        })?;
        if crls.is_empty() {
            let error_message = format!(
                "No CRLs found in {label} certificate revocation list file {:?}",
                path
            );
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
        Some(crls)
    } else {
        None
    };

    let maybe_identity = if let Some(path) = identity_path {
        let bytes = tokio::fs::read(path).await?;
        let identity = Identity::from_pem(&bytes).map_err(|error| {
            let error_message = format!(
                "Error getting {label} client identity at path {:?}: {}",
                path,
                error_root_cause(&error)
            );
            error!("{error_message}");
            DatapipeError::ValidationError(error_message)
        })?;
        Some(identity)
    } else {
        None
    };

    Ok((maybe_root_certs, maybe_crls, maybe_identity))
}

fn read_tls_certificates(
    path: &Path,
    label: &str,
) -> Result<Vec<CertificateDer<'static>>, DatapipeError> {
    let bytes = std::fs::read(path).map_err(|error| {
        let error_message = format!(
            "Cannot open TLS {label} file: {:?}: {}",
            path,
            error_root_cause(&error)
        );
        error!("{error_message}");
        DatapipeError::InputOutputError(error_message)
    })?;
    if bytes.is_empty() {
        let error_message = format!("TLS {label} file {:?} is empty", path);
        error!("{error_message}");
        return Err(DatapipeError::ValidationError(error_message));
    }

    let mut certs = Vec::new();
    for maybe_cert in CertificateDer::pem_slice_iter(&bytes) {
        match maybe_cert {
            Ok(cert) => certs.push(cert),
            Err(error) => {
                let error_message = format!(
                    "Error parsing PEM certificate from {label} file {:?}: {}",
                    path,
                    error_root_cause(&error)
                );
                error!("{error_message}");
                return Err(DatapipeError::ValidationError(error_message));
            }
        }
    }

    if !certs.is_empty() {
        return Ok(certs);
    }

    if contains_pem_marker(&bytes) {
        let error_message = format!("No certificates found in PEM {label} file {:?}", path);
        error!("{error_message}");
        return Err(DatapipeError::ValidationError(error_message));
    }

    // Fallback to single DER-encoded X.509 certificate (must begin with ASN.1 SEQUENCE tag 0x30)
    if bytes.first() != Some(&0x30) {
        let error_message = format!(
            "Invalid TLS {label} file {:?}: must be PEM or DER encoded X.509 certificate(s)",
            path
        );
        error!("{error_message}");
        return Err(DatapipeError::ValidationError(error_message));
    }

    Ok(vec![CertificateDer::from(bytes)])
}

// helper functions for TLS certificates and keys
fn get_root_ca(
    tls_root_ca_path: &Path,
    root_cert_store: &mut RootCertStore,
) -> Result<(), DatapipeError> {
    let certs = read_tls_certificates(tls_root_ca_path, "root CA")?;
    let mut added_count = 0usize;
    for ca in certs {
        match root_cert_store.add(ca) {
            Ok(()) => {
                added_count += 1;
            }
            Err(error) => {
                let error_message = format!(
                    "Error adding certificate authority (CA) to root cert store: {}",
                    error_root_cause(&error)
                );
                error!("{error_message}");
                return Err(DatapipeError::ValidationError(error_message));
            }
        }
    }
    if added_count == 0 {
        let error_message = format!(
            "No valid root CA certificates found in file {:?}",
            tls_root_ca_path
        );
        error!("{error_message}");
        return Err(DatapipeError::ValidationError(error_message));
    }
    Ok(())
}

fn get_tls_cert_chain(
    tls_cert_chain_path: &Path,
) -> Result<Vec<CertificateDer<'static>>, DatapipeError> {
    read_tls_certificates(tls_cert_chain_path, "certificate chain")
}

fn get_tls_private_key(
    tls_private_key_path: &Path,
) -> Result<PrivateKeyDer<'static>, DatapipeError> {
    let bytes = std::fs::read(tls_private_key_path).map_err(|error| {
        let error_message = format!(
            "Cannot open TLS private key file: {:?}: {}",
            tls_private_key_path,
            error_root_cause(&error)
        );
        error!("{error_message}");
        DatapipeError::InputOutputError(error_message)
    })?;
    if bytes.is_empty() {
        let error_message = format!("TLS private key file {:?} is empty", tls_private_key_path);
        error!("{error_message}");
        return Err(DatapipeError::ValidationError(error_message));
    }

    match PrivateKeyDer::from_pem_slice(&bytes) {
        Ok(private_key_der) => return Ok(private_key_der.clone_key()),
        Err(rustls_pki_types::pem::Error::NoItemsFound) => {
            if contains_pem_marker(&bytes) {
                let error_message = format!(
                    "Private key not found in PEM file: {:?}",
                    tls_private_key_path
                );
                error!("{error_message}");
                return Err(DatapipeError::ValidationError(error_message));
            }
        }
        Err(error) => {
            let error_message = format!(
                "Invalid or corrupted TLS private key file: {:?}: {}",
                tls_private_key_path,
                error_root_cause(&error)
            );
            error!("{error_message}");
            return Err(DatapipeError::ValidationError(error_message));
        }
    }

    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();
    let der_candidates = [
        PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(bytes.as_slice())),
        PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(bytes.as_slice())),
        PrivateKeyDer::Sec1(PrivateSec1KeyDer::from(bytes.as_slice())),
    ];
    for candidate in der_candidates {
        if tokio_rustls::rustls::crypto::ring::sign::any_supported_type(&candidate).is_ok() {
            return Ok(candidate.clone_key());
        }
    }

    let error_message = format!(
        "Invalid or unsupported TLS private key file {:?}: must be PEM or DER (PKCS#1, PKCS#8, or SEC1)",
        tls_private_key_path
    );
    error!("{error_message}");
    Err(DatapipeError::ValidationError(error_message))
}

/// Create a custom NO-OP verifier to allow --tls-skip-server-verify to work
/// NOTE:  this is DANGEROUS and should not be used in production!
#[derive(Debug)]
struct NoCertificateVerification {}

impl NoCertificateVerification {
    pub fn new() -> Self {
        Self {}
    }
}

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, tokio_rustls::rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA1,
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = version_info();
        assert!(info.contains(env!("CARGO_PKG_NAME")));
        assert!(info.contains(env!("CARGO_PKG_VERSION")));
        assert!(info.contains("Copyright"));
        assert!(info.contains("Richard Scott McNew"));
    }

    #[test]
    fn test_license_info() {
        let lic = license_info();
        assert!(lic.contains("GNU AFFERO GENERAL PUBLIC LICENSE"));
        assert!(lic.contains("Version 3, 19 November 2007"));
    }

    #[test]
    fn test_validate_version_and_license() {
        let mut args = ProgramArgs {
            version: true,
            ..Default::default()
        };
        assert!(args.validate().is_ok());

        args.version = false;
        args.license = true;
        assert!(args.validate().is_ok());
    }

    #[test]
    fn test_tls_helpers_accept_pem_and_der_and_reject_empty() {
        let cert_key = generate_simple_self_signed(vec!["localhost".to_string()])
            .expect("self-signed generation");
        let dir = std::env::temp_dir();
        let suffix = crate::datapipe_types::generate_random_string(12);

        let cert_pem = dir.join(format!("dp_cert_{suffix}.pem"));
        let cert_der = dir.join(format!("dp_cert_{suffix}.der"));
        let key_pem = dir.join(format!("dp_key_{suffix}.pem"));
        let key_der = dir.join(format!("dp_key_{suffix}.der"));
        let empty_file = dir.join(format!("dp_empty_{suffix}.bin"));

        std::fs::write(&cert_pem, cert_key.cert.pem()).expect("write cert pem");
        std::fs::write(&cert_der, cert_key.cert.der().as_ref()).expect("write cert der");
        std::fs::write(&key_pem, cert_key.signing_key.serialize_pem()).expect("write key pem");
        std::fs::write(&key_der, cert_key.signing_key.serialize_der()).expect("write key der");
        std::fs::write(&empty_file, []).expect("write empty");

        let pem_certs = get_tls_cert_chain(&cert_pem).expect("load PEM cert");
        let der_certs = get_tls_cert_chain(&cert_der).expect("load DER cert");
        assert_eq!(pem_certs.len(), 1);
        assert_eq!(der_certs.len(), 1);
        assert!(get_tls_cert_chain(&empty_file).is_err());

        assert!(get_tls_private_key(&key_pem).is_ok());
        assert!(get_tls_private_key(&key_der).is_ok());
        assert!(get_tls_private_key(&empty_file).is_err());

        let mut store = RootCertStore::empty();
        assert!(get_root_ca(&cert_pem, &mut store).is_ok());
        assert!(get_root_ca(&cert_der, &mut store).is_ok());
        assert!(get_root_ca(&empty_file, &mut store).is_err());

        for p in [cert_pem, cert_der, key_pem, key_der, empty_file] {
            let _ = std::fs::remove_file(p);
        }
    }
}
