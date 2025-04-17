use clap::{Args, Parser};
use crate::datapipe_types::{DatapipeError, EncryptionKey, error_root_cause};
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::engine::Parameters;
use crate::file_reader::FileReader;
use crate::http_reader::HttpReader;
use crate::https_reader::HttpsReader;
use crate::reader::Reader;
use crate::stdin_reader::StdinReader;
use crate::tcp_listen_reader::TcpListenReader;
use crate::tcp_reader_writer::TcpReaderWriter;
use crate::udp_reader::UdpReader;
use crate::file_writer::FileWriter;
use crate::http_writer::HttpWriter;
use crate::https_writer::HttpsWriter;
use crate::stdout_writer::StdoutWriter;
use crate::tls_reader_writer::TlsReaderWriter;
use crate::udp_writer::UdpWriter;
use crate::writer::Writer;
use log::{error, info};
use std::path::PathBuf;
use reqwest::{Certificate, tls::CertificateRevocationList, Identity};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;
use tokio_rustls::rustls::client::danger::{
    HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
};
use tokio_rustls::rustls::client::WantsClientCert;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use tokio_rustls::rustls::{
    ClientConfig, ConfigBuilder, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use webpki_roots::TLS_SERVER_ROOTS;



/// Choose one input source
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct InputArgs {
    /// read data from a file
    #[arg(long = "file-input")]
    pub file_input: Option<PathBuf>,
    /// read data from a HTTP URL; requires --http-input-rate
    #[arg(long = "http-input")]
    pub http_input: Option<String>,
    /// read data from a HTTPS URL; requires --https-input-rate and optionally server and client certificates
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
    /// read data from UDP address
    #[arg(long = "udp-input")]
    pub udp_input: Option<String>,
    /// read data from a UDP multicast address
    #[arg(long = "udp-multicast-input")]
    pub udp_multicast_input: Option<String>,
}

/// Additional parameters for HTTP input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpInputArgs {
    /// read rate in milliseconds, how often should the input web address be polled?
    #[arg(long = "http-input-rate")]
    pub http_input_rate: Option<u64>,    
}

/// Additional parameters for HTTPS input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
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
    /// path to client's custom private key and X509 certificate in PEM format.  Private key must be in RSA, SEC1 Elliptic Curve, or PKCS#8 format.
    #[arg(long = "https-input-client-identity")]
    pub https_input_client_identity: Option<PathBuf>,
    /// DANGER! Do not validate hostnames in HTTPS setup.  Use with caution.  DANGER!
    #[arg(long = "https-input-allow-invalid-hostnames")]
    pub https_input_allow_invalid_hostnames: Option<bool>,
    /// DANGER! Do not validate certificates in HTTPS setup.  Use with caution. DANGER!
    #[arg(long = "https-input-accept-invalid-certificates")]
    pub https_input_allow_invalid_certificates: Option<bool>,
}

/// Additional parameters needed for TLS input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct TlsInputArgs {
    /// path to custom TLS certificate chain file to use
    #[arg(long = "tls-input-cert-chain")]
    pub tls_input_cert_chain: Option<PathBuf>,
    /// path to custom TLS client certificate to use
    #[arg(long = "tls-input-client-key")]
    pub tls_input_client_key: Option<PathBuf>,
    /// path to custom Certificate Authority to use instead of web root CAs
    #[arg(long = "tls-input-root-ca")]
    pub tls_input_root_ca: Option<PathBuf>,
    /// DANGER! Do not validate server identity.  Use with caution. DANGER!
    #[arg(long = "tls-input-skip-server-verify", default_value_t = false)]
    pub tls_input_skip_server_verify: bool, 
}


/// Additional parameters for decrypting input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = false)]
pub struct DecryptionArgs {
    /// decryption key to use after reading data; must be exactly 51 bytes long
    #[arg(long = "decrypt")]
    pub decryption_key: Option<String>,
}

/// Additional parameters for encrypting output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = false)]
pub struct EncryptionArgs {
    /// encryption key to use before writing data; must be exactly 51 bytes long
    #[arg(long = "encrypt")]
    pub encryption_key: Option<String>,
    /// generate an encryption key
    #[arg(long = "encrypt-generate-key", default_value_t = false)]
    pub generate_encryption_key: bool,
}


/// Choose one or more output destinations
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = true)]
pub struct OutputArgs {    
    /// write data to a file
    #[arg(long = "file-output")]
    pub file_output: Option<PathBuf>,    
    /// write data to a HTTP URL; requires http-output-rate and optionally http-output-delimiter and http-output-include-delimiter
    #[arg(long = "http-output")]
    pub http_output: Option<String>,
    /// write data to a HTTPS URL; requires https-output-rate and optionally https-output-delimiter, https-output-include-delimiter, and custom server / client certificates
    #[arg(long = "https-output")]
    pub https_output: Option<String>,
    /// write data to STDOUT
    #[arg(long = "stdout-output")]
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
}

/// Additional parameters needed for HTTP output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpOutputArgs {
    /// write rate in milliseconds, how often should data be sent to the output web address?
    #[arg(long = "http-output-rate")]
    pub http_output_rate: Option<u64>,    
    /// this delimiter is used to group the output into one or more 'segments' that will be sent in each request; defaults to newline
    #[arg(long = "http-output-delimiter")]
    pub http_output_delimiter: Option<Vec<u8>>,
    /// should the delimiter be included with the segment that preceeds it?  defaults to true
    #[arg(long = "http-output-include-delimiter")]
    pub http_output_include_delimiter: Option<bool>,
}

/// Additional parameters for HTTPS output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
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
    #[arg(long = "https-output-allow-invalid-hostnames")]
    pub https_output_allow_invalid_hostnames: Option<bool>,
    /// DANGER! Do not validate certificates in HTTPS setup.  Use with caution. DANGER!
    #[arg(long = "https-output-allow-invalid-certificates")]
    pub https_output_allow_invalid_certificates: Option<bool>,
}

/// Additional parameters needed for TLS output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct TlsOutputArgs {
    /// path to custom TLS certificate chain file to use
    #[arg(long = "tls-output-cert-chain")]
    pub tls_output_cert_chain: Option<PathBuf>,
    /// path to custom TLS client certificate to use
    #[arg(long = "tls-output-client-key")]
    pub tls_output_client_key: Option<PathBuf>,
    /// path to custom Certificate Authority certificates to use instead of web root CAs
    #[arg(long = "tls-output-root-ca")]
    pub tls_output_root_ca: Option<PathBuf>,
    /// DANGER! Do not validate server identity.  Use with caution. DANGER!
    #[arg(long = "tls-output-skip-server-verify")]
    pub tls_output_skip_server_verify: Option<bool>, 
}

/// Logging parameters
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct LoggingArgs {
    /// should logs be kept after the program exits?  defaults to false
    #[arg(long = "keep-logs", default_value_t = false)]
    pub keep_logs: bool,
    /// where should logs be written?  defaults to /var/tmp
    #[arg(long = "log-dir")]
    pub log_dir: Option<String>,    
}


/// Overall command line args
#[derive(Parser, Debug, Clone)]
pub struct ProgramArgs {
    #[command(flatten)]
    pub input: InputArgs,
    #[command(flatten)]
    pub http_input: HttpInputArgs,
    #[command(flatten)]
    pub https_input: HttpsInputArgs,
    #[command(flatten)]
    pub tls_input: TlsInputArgs,
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
    pub logging_args: LoggingArgs,
}

impl ProgramArgs {

    /// Ensure that the input reader is set only once
    fn check_reader_set(maybe_reader: &Option<Reader>) -> Result<(), DatapipeError> {
        match maybe_reader.as_ref() {
            Some(reader) => {
                let error_message = format!("Input previously assigned as {:?}; only one input can be used.", reader);
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }        
            None => Ok(())
        }    
    }

    /// Prepare a reader for file input
    async fn handle_file_input(&self) -> Result<Reader, DatapipeError> {    
        let file_path = self.input.file_input.as_ref().unwrap(); // is_some checked in parent function
        let file_reader = FileReader::new(file_path).await?;
        info!("Using FILE input");
        Ok(Reader::File(file_reader))        
    }

    /// Prepare a reader for HTTP input
    fn handle_http_input(&self) -> Result<Reader, DatapipeError> {
        let url = self.input.http_input.as_ref().unwrap();  // is_some checked in parent function
        let update_rate;
        if self.http_input.http_input_rate.is_some() {        
            update_rate = self.http_input.http_input_rate.unwrap();
            info!("Using HTTP input rate of {} milliseconds", update_rate);
        } else {        
            update_rate = HttpReader::DEFAULT_UPDATE_RATE;
            info!("Using default HTTP input rate of {} milliseconds", update_rate);
        }
        let http_reader = HttpReader::new(url, update_rate)?;
        info!("Using HTTP input");
        Ok(Reader::Http(http_reader))
    }

    /// Prepare a reader for HTTPS input
    async fn handle_https_input(&self) -> Result<Reader, DatapipeError> {
        let url = self.input.https_input.as_ref().unwrap();  // is_some checked in parent function    
        let read_rate;
        let maybe_root_certs;
        let maybe_crls;
        let maybe_identity;
        
        let allow_invalid_hostnames = *self.https_input.https_input_allow_invalid_hostnames.as_ref().unwrap_or(&false);       
        let allow_invalid_certs = *self.https_input.https_input_allow_invalid_certificates.as_ref().unwrap_or(&false);

        if self.https_input.https_input_rate.is_some() {
            let read_rate_millis = self.https_input.https_input_rate.unwrap();
            read_rate = Duration::from_millis(read_rate_millis);
        } else {
            read_rate = HttpsReader::DEFAULT_READ_RATE;
        }

        if self.https_input.https_input_root_certificates.is_some() {
            let root_cert_path = self.https_input.https_input_root_certificates.clone().unwrap();
            let root_cert_bytes = tokio::fs::read(&root_cert_path).await?;
            match reqwest::Certificate::from_pem_bundle(&root_cert_bytes) {
                Ok(root_certs) => {
                    maybe_root_certs = Some(root_certs);
                }
                Err(error) => {
                    let error_message = format!("Error getting HTTPS input root certificate at path {:?}: {}", root_cert_path, error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }        
        } else {
            maybe_root_certs = None;
        }

        if self.https_input.https_input_certificate_revocation_list.is_some() {
            let crl_path = self.https_input.https_input_certificate_revocation_list.clone().unwrap();
            let crl_bytes = tokio::fs::read(&crl_path).await?;
            match reqwest::tls::CertificateRevocationList::from_pem_bundle(&crl_bytes) {
                Ok(crls) => {
                    maybe_crls = Some(crls);
                }
                Err(error) => {
                    let error_message = format!("Error getting HTTPS input certificate revocation list at path {:?}: {}", crl_path, error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }        
        } else {
            maybe_crls = None;
        }

        if self.https_input.https_input_client_identity.is_some() {
            let identity_path = self.https_input.https_input_client_identity.clone().unwrap();
            let identity_bytes = tokio::fs::read(&identity_path).await?;
            match reqwest::tls::Identity::from_pem(&identity_bytes) {
                Ok(identity) => {
                    maybe_identity = Some(identity);
                }
                Err(error) => {
                    let error_message = format!("Error getting HTTPS input client identity at path {:?}: {}", identity_path, error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }        
        } else {
            maybe_identity = None;
        }

        let https_reader = HttpsReader::new(url, read_rate, maybe_root_certs, maybe_crls, maybe_identity, allow_invalid_hostnames, allow_invalid_certs)?;
        info!("Using HTTPS input");
        Ok(Reader::Https(https_reader))
    }

    /// Prepare a reader for STDIN input
    fn handle_stdin_input(&self) -> Reader {
        info!("Using STDIN input");
        Reader::Stdin(StdinReader::new())
    }

    async fn handle_tcp_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.tcp_input.as_ref().unwrap();
        match TcpReaderWriter::new(address).await {
            Ok(tcp_reader) => {
                Ok(Reader::Tcp(tcp_reader))
            }
            Err(error) => {
                let error_message = format!("TCP input error {}: {}", &address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    async fn handle_tcp_listen_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.tcp_listen_input.as_ref().unwrap();
        match TcpListenReader::new(address).await {
            Ok(tcp_listen_reader) => {
                Ok(Reader::TcpListen(tcp_listen_reader))
            }
            Err(error) => {
                let error_message = format!("TCP listen input error {}: {}", &address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    async fn handle_tls_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.tls_input.as_ref().unwrap();        
        match self.get_tls_input_config() {
            Ok(tls_config) => match TlsReaderWriter::new(address.to_owned(), tls_config).await {
                Ok(tls_reader) => {
                    info!("Using TLS input");
                    Ok(Reader::Tls(tls_reader))                
                }
                Err(error) => {
                    let error_message = format!("TLS input error {}: {}", &address, error_root_cause(&error));
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

    fn get_tls_input_config(&self) -> Result<ClientConfig, DatapipeError> {
        // setup root cert store
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
        if self.tls_input.tls_input_root_ca.is_some() {
            match get_root_ca(
                self.tls_input.tls_input_root_ca.as_ref().unwrap(),
                &mut root_cert_store,
            ) {
                Ok(()) => {} // no issues loading CA roots
                Err(error) => {
                    let error_message =
                        format!("Error loading TLS input certificate authority (CA) roots: {}", error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }
        }
        // begin build the config
        // check if no verification was requested
        let config: ConfigBuilder<ClientConfig, WantsClientCert> =
            if self.tls_input.tls_input_skip_server_verify {
                let dangerous_config = ConfigBuilder::dangerous(ClientConfig::builder());
                dangerous_config
                    .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new()))
            } else {
                ClientConfig::builder().with_root_certificates(root_cert_store)
            };
        // see if client auth is needed
        match self.tls_input.tls_input_cert_chain.as_ref() {
            Some(tls_cert_chain_path) => {
                // get certificate chain
                match get_tls_cert_chain(tls_cert_chain_path) {
                    Ok(cert_chain) => {
                        // get client certificate
                        match self.tls_input.tls_input_client_key.as_ref() {
                            Some(tls_client_key_path) => {
                                match get_tls_client_key(tls_client_key_path) {
                                    Ok(client_key) => {
                                        // finish building the config with cert chain and client key
                                        match config.with_client_auth_cert(cert_chain, client_key) {
                                            Ok(tls_config) => {
                                                return Ok(tls_config);
                                            }
                                            Err(error) => {
                                                let error_message = format!("Error creating TLS input config with cert chain and client key: {}", error_root_cause(&error));
                                                error!("{error_message}");
                                                return Err(DatapipeError::InputOutputError(error_message));
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        // failed to get client key
                                        let error_message = format!(
                                            "Error getting TLS input client key {:?}: {}",
                                            tls_client_key_path, error_root_cause(&error)
                                        );
                                        error!("{error_message}");
                                        return Err(DatapipeError::InputOutputError(error_message));
                                    }
                                }
                            }
                            None => {
                                // the user should have provided a client key too
                                let error_message = "TLS input certificate chain (--tls-input-cert-chain) requires client key (--tls-input-client-key) to also be used";
                                error!("{error_message}");
                                return Err(DatapipeError::ValidationError(error_message.to_string()));
                            }
                        }
                    }
                    Err(error) => {
                        // failed to get cert chain
                        let error_message = format!(
                            "Error getting TLS input certificate chain {:?}: {}",
                            tls_cert_chain_path, error_root_cause(&error)
                        );
                        error!("{error_message}");
                        return Err(DatapipeError::InputOutputError(error_message));
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
                // finishing build the config with no client authentication
                return Ok(config.with_no_client_auth());
            }
        }
    }

    /// Prepare a reader for UDP input
    async fn handle_udp_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.udp_input.as_ref().unwrap();  // is_some checked in parent function
        match UdpReader::new(address).await {
            Ok(udp_reader) => {            
                info!("Using UDP input");
                Ok(Reader::Udp(udp_reader))
            }
            Err(error) => {
                let error_message =
                    format!("Cannot open input UDP address {:?}: {}", &address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    /// Prepare a reader for UDP multicast input
    async fn handle_udp_multicast_input(&self) -> Result<Reader, DatapipeError> {
        let address = self.input.udp_multicast_input.as_ref().unwrap();
        match UdpReader::new_multicast(address).await {
            Ok(udp_reader) => {            
                info!("Using UDP multicast input");
                Ok(Reader::Udp(udp_reader))
            }
            Err(error) => {
                let error_message =
                    format!("Cannot open input UDP multicast address {:?}: {}", &address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    /// Select the wanted input implementation from the command line args
    async fn get_input_reader(&self) -> Result<Reader, DatapipeError> {    
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
        if self.input.udp_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_udp_input().await?);        
        }
        if self.input.udp_multicast_input.is_some() {
            Self::check_reader_set(&maybe_reader)?;
            maybe_reader = Some(self.handle_udp_multicast_input().await?);        
        }    
        match maybe_reader {
            Some(reader) => Ok(reader),
            None => {
                let error_message = "No input source provided!";
                error!("{error_message}");
                return Err(DatapipeError::ValidationError(error_message.to_string()));
            }
        }    
    }

    async fn handle_file_output(&self) -> Result<Writer, DatapipeError> {
        let file_path = self.output.file_output.as_ref().unwrap();        
        match FileWriter::new(&file_path).await {
            Ok(file_writer) => {            
                info!("Using FILE output for path: {:?}", file_path);
                Ok(Writer::File(file_writer))
            }
            Err(error) => {
                let error_message = format!("File output error {:?}: {}", file_path, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    fn handle_http_output(&self) -> Result<Writer, DatapipeError> {
        let url = self.output.http_output.as_ref().unwrap();        
        let output_rate: Duration;
        let delimiter: Vec<u8>;
        let include_delimiter: bool;
        if self.http_output.http_output_delimiter.is_some() {
            delimiter = self.http_output.http_output_delimiter.as_ref().unwrap().to_vec();
        } else {
            delimiter = HttpWriter::DEFAULT_DELIMITER.to_vec();
        }
        if self.http_output.http_output_include_delimiter.is_some() {
            include_delimiter = self.http_output.http_output_include_delimiter.unwrap();
        } else {
            include_delimiter = true;
        }
        if self.http_output.http_output_rate.is_some() {
            output_rate = Duration::from_millis(self.http_output.http_output_rate.unwrap());
        } else {
            output_rate = HttpWriter::DEFAULT_WRITE_RATE;
        }
        match HttpWriter::new(&url, delimiter, include_delimiter, output_rate) {            
            Ok(http_writer) => {
                info!("Using HTTP output");
                Ok(Writer::Http(http_writer))            
            }
            Err(error) => {
                let error_message = format!("HTTP URL error {}: {}", &url, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        }
    }

    async fn handle_https_output(&self) -> Result<Writer, DatapipeError> {
        let url = self.output.http_output.as_ref().unwrap();        
        let write_rate;
        let delimiter: Vec<u8>;
        let include_delimiter: bool;
        let maybe_root_certs: Option<Vec<Certificate>>;
        let maybe_crls: Option<Vec<CertificateRevocationList>>;
        let maybe_identity: Option<Identity>;

        let allow_invalid_hostnames = *self.https_output.https_output_allow_invalid_hostnames.as_ref().unwrap_or(&false);       
        let allow_invalid_certs = *self.https_output.https_output_allow_invalid_certificates.as_ref().unwrap_or(&false);

        if self.https_output.https_output_rate.is_some() {
            let write_rate_millis = self.https_output.https_output_rate.unwrap();
            write_rate = Duration::from_millis(write_rate_millis);
        } else {
            write_rate = HttpsWriter::DEFAULT_WRITE_RATE;
        }

        if self.https_output.https_output_delimiter.is_some() {
            delimiter = self.https_output.https_output_delimiter.as_ref().unwrap().to_vec();
        } else {
            delimiter = HttpsWriter::DEFAULT_DELIMITER.to_vec();
        }
        if self.https_output.https_output_include_delimiter.is_some() {
            include_delimiter = self.https_output.https_output_include_delimiter.unwrap();
        } else {
            include_delimiter = true;
        }
        
        if self.https_output.https_output_root_certificates.is_some() {
            let root_cert_path = self.https_output.https_output_root_certificates.clone().unwrap();
            let root_cert_bytes = tokio::fs::read(&root_cert_path).await?;
            match reqwest::Certificate::from_pem_bundle(&root_cert_bytes) {
                Ok(root_certs) => {
                    maybe_root_certs = Some(root_certs);
                }
                Err(error) => {
                    let error_message = format!("Error getting HTTPS output root certificate at path {:?}: {}", root_cert_path, error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }        
        } else {
            maybe_root_certs = None;
        }

        if self.https_output.https_output_certificate_revocation_list.is_some() {
            let crl_path = self.https_output.https_output_certificate_revocation_list.clone().unwrap();
            let crl_bytes = tokio::fs::read(&crl_path).await?;
            match reqwest::tls::CertificateRevocationList::from_pem_bundle(&crl_bytes) {
                Ok(crls) => {
                    maybe_crls = Some(crls);
                }
                Err(error) => {
                    let error_message = format!("Error getting HTTPS output certificate revocation list at path {:?}: {}", crl_path, error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }        
        } else {
            maybe_crls = None;
        }

        if self.https_output.https_output_client_identity.is_some() {
            let identity_path = self.https_output.https_output_client_identity.clone().unwrap();
            let identity_bytes = tokio::fs::read(&identity_path).await?;
            match reqwest::tls::Identity::from_pem(&identity_bytes) {
                Ok(identity) => {
                    maybe_identity = Some(identity);
                }
                Err(error) => {
                    let error_message = format!("Error getting HTTPS output client identity at path {:?}: {}", identity_path, error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }        
        } else {
            maybe_identity = None;
        }

        match HttpsWriter::new(&url, delimiter, include_delimiter, write_rate, maybe_root_certs, maybe_crls, maybe_identity, allow_invalid_hostnames, allow_invalid_certs) {            
            Ok(https_writer) => {
                info!("Using HTTPS output");
                Ok(Writer::Https(https_writer))            
            }
            Err(error) => {
                let error_message = format!("HTTPS output URL error {}: {}", &url, error_root_cause(&error));
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
        let address = self.output.tcp_output.as_ref().unwrap();
        match TcpReaderWriter::new(address).await {
            Ok(tcp_writer) => {
                Ok(Writer::Tcp(tcp_writer))
            }
            Err(error) => {
                let error_message = format!("TCP output error {}: {}", &address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }
    }

    async fn handle_tls_output(&self) -> Result<Writer, DatapipeError> {
        let address = self.output.tls_output.as_ref().unwrap();        
        let tls_config = self.get_tls_output_config()?;        
        match TlsReaderWriter::new(address.to_owned(), tls_config).await {
            Ok(tls_writer) => {
                info!("Using TLS output");
                Ok(Writer::Tls(tls_writer))                
            }
            Err(error) => {
                let error_message = format!("TLS output error {}: {}", &address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::InputOutputError(error_message))
            }
        
        }
    }

    fn get_tls_output_config(&self) -> Result<ClientConfig, DatapipeError> {
        // setup root cert store
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
        if self.tls_output.tls_output_root_ca.is_some() {
            match get_root_ca(
                self.tls_output.tls_output_root_ca.as_ref().unwrap(),
                &mut root_cert_store,
            ) {
                Ok(()) => {} // no issues loading CA roots
                Err(error) => {
                    let error_message =
                        format!("Error loading TLS output certificate authority (CA) roots: {}", error_root_cause(&error));
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }
        }
        // begin build the config
        // check if no verification was requested
        let config: ConfigBuilder<ClientConfig, WantsClientCert> =
            if self.tls_output.tls_output_skip_server_verify.unwrap_or(false) {
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
                        let client_key = get_tls_client_key(tls_client_key_path)?;                                    
                        // finish building the config with cert chain and client key
                        match config.with_client_auth_cert(cert_chain, client_key) {
                            Ok(tls_config) => {
                                return Ok(tls_config);
                            }
                            Err(error) => {
                                let error_message = format!("Error creating TLS output config with cert chain and client key: {}", error_root_cause(&error));
                                error!("{error_message}");
                                return Err(DatapipeError::ValidationError(error_message));
                            }
                        }
                    }
                    None => {
                        // the user should have provided a client key too
                        let error_message = "TLS output certificate chain (--tls-output-cert-chain) requires client key (--tls-output-client-key) to also be used";
                        error!("{error_message}");
                        return Err(DatapipeError::ValidationError(error_message.to_string()));
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
                return Ok(config.with_no_client_auth());
            }
        }
    }

    async fn handle_udp_output(&self) -> Result<Writer, DatapipeError> {
        let address = self.output.udp_output.as_ref().unwrap();        
        match UdpWriter::new(&address).await {
            Ok(udp_writer) => {
                info!("Using UDP output");
                Ok(Writer::Udp(udp_writer))            
            }
            Err(error) => {
                let error_message = format!("UDP output error {}: {}", address, error_root_cause(&error));
                error!("{error_message}");
                Err(DatapipeError::ValidationError(error_message))
            }
        }        
    }

    async fn get_output_writers(&self) -> Result<Vec<Writer>, DatapipeError> {
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
        if writers.is_empty() {
            let error_message = "No output destination provided!";
            error!("{error_message}");
            Err(DatapipeError::ValidationError(error_message.to_string()))
        } else {
            Ok(writers)
        }
    }

    
    fn get_encryption_args(&self) -> Result<Option<StreamEncryptor>, DatapipeError> {        
        if self.encryption_args.generate_encryption_key {
            let encryption_key = EncryptionKey::generate();
            println!("Generated encryption key: {}", encryption_key.to_string());
            let encryptor = StreamEncryptor::new(encryption_key)?;
            return Ok(Some(encryptor));
        } 
        if self.encryption_args.encryption_key.is_some() {
            let encryption_key = EncryptionKey::new(self.encryption_args.encryption_key.as_ref().unwrap()).unwrap();
            let encryptor = StreamEncryptor::new(encryption_key)?;
            return Ok(Some(encryptor));
        }
        Ok(None)
    }

    fn get_decryption_args(&self) -> Result<Option<StreamDecryptor>, DatapipeError> {
        if self.decryption_args.decryption_key.is_some() {
            let encryption_key = EncryptionKey::new(self.decryption_args.decryption_key.as_ref().unwrap()).unwrap();
            let decryptor = StreamDecryptor::new(encryption_key)?;
            return Ok(Some(decryptor));
        }
        Ok(None)
    }

    pub async fn to_parameters(&self) -> Result<Parameters, DatapipeError> {
        let reader = self.get_input_reader().await?;
        let writers = self.get_output_writers().await?;
        let maybe_decryptor = self.get_decryption_args()?;
        let maybe_encryptor = self.get_encryption_args()?;
        
        Ok(Parameters{
            reader,
            maybe_decryptor,
            maybe_encryptor,
            writers
        })
    }
}


// helper functions for TLS certificates and keys
fn get_root_ca(
    tls_root_ca_path: &PathBuf,
    root_cert_store: &mut RootCertStore,
) -> Result<(), DatapipeError> {
    match File::open(tls_root_ca_path) {
        Ok(tls_root_ca_file) => {
            let mut root_ca_buffer = BufReader::new(tls_root_ca_file);
            for maybe_ca in certs(&mut root_ca_buffer) {
                match maybe_ca {
                    Ok(ca) => {
                        match root_cert_store.add(ca) {
                            Ok(()) => {
                                // successfully added, keep going
                            }
                            Err(error) => {
                                let error_message = format!("Error adding certificate authority (CA) to root cert store: {}", error_root_cause(&error));
                                error!("{error_message}");
                                return Err(DatapipeError::ValidationError(error_message));
                            }
                        }
                    }
                    Err(error) => {
                        let error_message = format!(
                            "Error parsing certificate authority (CA) from {:?}: {}",
                            &tls_root_ca_path, error_root_cause(&error)
                        );
                        error!("{error_message}");
                        return Err(DatapipeError::ValidationError(error_message));
                    }
                }
            }
        }
        Err(error) => {
            let error_message = format!(
                "Cannot open TLS root CA file: {:?}: {}",
                &tls_root_ca_path, error_root_cause(&error)
            );
            error!("{error_message}");
            return Err(DatapipeError::InputOutputError(error_message));
        }
    }
    Ok(())
}

fn get_tls_cert_chain(
    tls_cert_chain_path: &PathBuf,
) -> Result<Vec<CertificateDer<'static>>, DatapipeError> {
    let mut cert_chain = Vec::new();
    match File::open(tls_cert_chain_path) {
        Ok(tls_cert_chain_file) => {
            let mut cert_chain_buffer = BufReader::new(tls_cert_chain_file);
            for maybe_cert in certs(&mut cert_chain_buffer) {
                match maybe_cert {
                    Ok(cert) => {
                        cert_chain.push(cert);
                    }
                    Err(error) => {
                        let error_message =
                            format!("Error adding certificate to certificate chain: {}", error_root_cause(&error));
                        error!("{error_message}");
                        return Err(DatapipeError::InputOutputError(error_message));
                    }
                }
            }
        }
        Err(error) => {
            let error_message = format!(
                "Cannot open TLS certificate chain file: {:?}: {}",
                &tls_cert_chain_path, error_root_cause(&error)
            );
            error!("{error_message}");
            return Err(DatapipeError::InputOutputError(error_message));
        }
    }
    Ok(cert_chain)
}

fn get_tls_client_key(
    tls_client_key_path: &PathBuf,
) -> Result<PrivateKeyDer<'static>, DatapipeError> {
    let private_key_der: PrivateKeyDer<'static>;
    match File::open(tls_client_key_path) {
        Ok(tls_client_key_file) => {
            let mut client_key_buffer = BufReader::new(tls_client_key_file);
            match private_key(&mut client_key_buffer) {
                Ok(maybe_private_key_der) => match maybe_private_key_der {
                    Some(der) => {
                        private_key_der = der;
                    }
                    None => {
                        let error_message = format!(
                            "Private key not found in file: {:?}; file must be in PEM format",
                            &tls_client_key_path
                        );
                        error!("{error_message}");
                        return Err(DatapipeError::ValidationError(error_message));
                    }
                },
                Err(error) => {
                    let error_message = format!(
                        "Invalid or corrupted TLS client key file: {:?}: {}",
                        &tls_client_key_path, error_root_cause(&error)
                    );
                    error!("{error_message}");
                    return Err(DatapipeError::ValidationError(error_message));
                }
            }
        }
        Err(error) => {
            let error_message = format!(
                "Cannot open TLS client key file: {:?}: {}",
                &tls_client_key_path, error_root_cause(&error)
            );
            error!("{error_message}");
            return Err(DatapipeError::InputOutputError(error_message));
        }
    }
    Ok(private_key_der)
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



