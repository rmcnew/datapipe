use clap::{Args, Parser};
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
use crate::stdout_writer::StdoutWriter;
use crate::tls_reader_writer::TlsReaderWriter;
use crate::udp_writer::UdpWriter;
use crate::writer::Writer;
use log::{error, info};
use std::path::PathBuf;
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::io;
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
    /// --file-input  read input from file
    #[arg(long = "file-input")]
    pub file_input: Option<PathBuf>,
    /// --http-input  read input from HTTP URL; requires --http-input-rate
    #[arg(long = "http-input")]
    pub http_input: Option<String>,
    /// --https-input  read input from HTTPS URL; requires --https-input-rate and optionally server and client certificates
    #[arg(long = "https-input")]
    pub https_input: Option<String>,    
    /// --stdin-input  read input from STDIN
    #[arg(long = "stdin-input", default_value_t = false)]
    pub stdin_input: bool,
    /// --tcp-input  read input from TCP address
    #[arg(long = "tcp-input")]
    pub tcp_input: Option<String>,
    /// --tcp-listen-input  open a local port to receive a TCP connection
    #[arg(long = "tcp-listen-input")]
    pub tcp_listen_input: Option<String>,
    /// --tls-input  read input from TLS address
    #[arg(long = "tls-input")]
    pub tls_input: Option<String>,
    /// --udp-input  read input from UDP address
    #[arg(long = "udp-input")]
    pub udp_input: Option<String>,   
}

/// Additional parameters for HTTP input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpInputArgs {
    /// --http-input-rate  in milliseconds, how often should the input web address be polled?
    #[arg(long = "http-input-rate")]
    pub http_input_rate: Option<u64>,    
}

/// Additional parameters for HTTPS input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpsInputArgs {
    /// --https-input-rate  in milliseconds, how often should the input web address be polled?
    #[arg(long = "https-input-rate")]
    pub https_input_rate: Option<u64>,    
    /// --https-input-root-certificate  path to custom root certificate file to use in PEM format
    #[arg(long = "https-input-root-certificate")]
    pub https_input_root_certificate: Option<PathBuf>,
    /// --https-input-certificate-revocation-list  path to custom certificate revocation list file to use in PEM format
    #[arg(long = "https-input-certificate-revocation-list")]
    pub https_input_certificate_revocation_list: Option<PathBuf>,
    /// --https-input-client-identity  path to client's custom private key and X509 certificate in PEM format.  Private key must be in RSA, SEC1 Elliptic Curve, or PKCS#8 format.
    #[arg(long = "https-input-client-identity")]
    pub https_input_client_identity: Option<PathBuf>,
    /// --https-input-accept-invalid-hostnames  DANGER! Do not validate hostnames in HTTPS setup.  Use with caution.  DANGER!
    #[arg(long = "https-input-accept-invalid-hostnames")]
    pub https_input_accept_invalid_hostnames: Option<bool>,
    /// --https-input-accept-invalid-certificates  DANGER! Do not validate certificates in HTTPS setup.  Use with caution. DANGER!
    #[arg(long = "https-input-accept-invalid-certificates")]
    pub https_input_accept_invalid_certificates: Option<bool>,
}

/// Additional parameters needed for TLS input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct TlsInputArgs {
    /// --tls-input-cert-chain  TLS certificate chain file to use
    #[arg(long = "tls-input-cert-chain")]
    pub tls_input_cert_chain: Option<PathBuf>,
    /// --tls-input-client-key  TLS client certificate to use
    #[arg(long = "tls-input-client-key")]
    pub tls_input_client_key: Option<PathBuf>,
    /// --tls-input-root-ca  use these CAs instead of web root CAs
    #[arg(long = "tls-input-root-ca")]
    pub tls_input_root_ca: Option<PathBuf>,
    /// --tls-input-skip-server-verify  DANGER! Do not validate server identity.  Use with caution. DANGER!
    #[arg(long = "tls-input-skip-server-verify", default_value_t = false)]
    pub tls_input_skip_server_verify: bool, 
}

/// Additional parameters for decrypting input
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct DecryptionArgs {
    /// --decrypt  decryption key to use
    #[arg(long = "decrypt")]
    pub decryption_key: Option<String>,
}

/// Additional parameters for encrypting output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct EncryptionArgs {
    /// --encrypt  encryption key to use
    #[arg(long = "encrypt")]
    pub encryption_key: Option<String>,
}

/// Choose one or more output destinations
#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = true)]
pub struct OutputArgs {    
    /// --file-output  write output to file
    #[arg(long = "file-output")]
    pub file_output: Option<Vec<PathBuf>>,    
    /// --http-output  write output to HTTP URL
    #[arg(long = "http-output")]
    pub http_output: Option<Vec<String>>,
    /// --https-output  write output to HTTPS URL; requires http-output-rate and optionally delimiter
    #[arg(long = "https-output")]
    pub https_output: Option<Vec<String>>,
    /// --stdout-output  write output to STDOUT
    #[arg(long = "stdout-output")]
    pub stdout_output: bool,
    /// --tcp-output  write output to TCP address
    #[arg(long = "tcp-output")]
    pub tcp_output: Option<Vec<String>>,
    /// --tls-output  write output to TCP/TLS URL
    #[arg(long = "tls-output")]
    pub tls_output: Option<Vec<String>>,
    /// --udp-output  write output to UDP address
    #[arg(long = "udp-output")]
    pub udp_output: Option<Vec<String>>,
}

/// Additional parameters needed for HTTP output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpOutputArgs {
    /// --http-output-rate  in milliseconds, how often should data be sent to the output web address?
    #[arg(long = "http-output-rate")]
    pub http_output_rate: Option<Vec<u64>>,    
    /// --http-output-delimiter  this delimiter is used to group the output into one or more 'segments' that will be sent in each request
    #[arg(long = "http-output-delimiter")]
    pub http_output_delimiter: Option<Vec<Vec<u8>>>,
    /// --http-output-include-delimiter  should the delimiter be included with the segment that preceeds it?
    #[arg(long = "http-output-include-delimiter")]
    pub http_output_include_delimiter: Option<Vec<bool>>,
}

/// Additional parameters for HTTPS output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct HttpsOutputArgs {
    /// --https-output-rate  in milliseconds, how often should the output web address be polled?
    #[arg(long = "https-output-rate")]
    pub https_output_rate: Option<Vec<u64>>,    
    /// --https-output-root-certificate  path to custom root certificate file to use in PEM format
    #[arg(long = "https-output-root-certificate")]
    pub https_output_root_certificate: Option<Vec<PathBuf>>,
    /// --https-output-certificate-revocation-list  path to custom certificate revocation list file to use in PEM format
    #[arg(long = "https-output-certificate-revocation-list")]
    pub https_output_certificate_revocation_list: Option<Vec<PathBuf>>,
    /// --https-output-client-identity  path to client's custom private key and X509 certificate in PEM format.  Private key must be in RSA, SEC1 Elliptic Curve, or PKCS#8 format.
    #[arg(long = "https-output-client-identity")]
    pub https_output_client_identity: Option<Vec<PathBuf>>,
    /// --https-output-accept-invalid-hostnames  DANGER! Do not validate hostnames in HTTPS setup.  Use with caution.  DANGER!
    #[arg(long = "https-output-accept-invalid-hostnames")]
    pub https_output_accept_invalid_hostnames: Option<Vec<bool>>,
    /// --https-output-accept-invalid-certificates  DANGER! Do not validate certificates in HTTPS setup.  Use with caution. DANGER!
    #[arg(long = "https-output-accept-invalid-certificates")]
    pub https_output_accept_invalid_certificates: Option<Vec<bool>>,
}

/// Additional parameters needed for TLS output
#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct TlsOutputArgs {
    /// --tls-output-cert-chain  TLS certificate chain file to use
    #[arg(long = "tls-output-cert-chain")]
    pub tls_output_cert_chain: Option<Vec<PathBuf>>,
    /// --tls-output-client-key  TLS client certificate to use
    #[arg(long = "tls-output-client-key")]
    pub tls_output_client_key: Option<Vec<PathBuf>>,
    /// --tls-output-root-ca  use these CAs instead of web root CAs
    #[arg(long = "tls-output-root-ca")]
    pub tls_output_root_ca: Option<Vec<PathBuf>>,
    /// --tls-output-skip-server-verify  DANGER! Do not validate server identity.  Use with caution. DANGER!
    #[arg(long = "tls-output-skip-server-verify")]
    pub tls_output_skip_server_verify: Option<Vec<bool>>, 
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
}

/// Ensure that the input reader is set only once
fn check_reader_set(maybe_reader: &Option<Reader>) -> Result<(), Error> {
    match maybe_reader.as_ref() {
        Some(reader) => {
            let error_message = format!("Input previously assigned as {:?}; only one input can be used.", reader);
            error!("{}", error_message);
            Err(Error::new(ErrorKind::AlreadyExists, error_message))
        }        
        None => Ok(())
    }    
}

/// Prepare a reader for file input
async fn handle_file_input(args: &ProgramArgs) -> Result<Reader, Error> {    
    let file_path = args.input.file_input.as_ref().unwrap(); // is_some checked in parent function
    match FileReader::new(file_path).await {
        Ok(file_reader) => {
            info!("Using FILE input");
            Ok(Reader::File(file_reader))            
        }
        Err(error) => {
            let error_message = format!("File input error for '{:?}': {}", file_path, error);
            error!("{}", error_message);
            Err(Error::new(error.kind(), error_message))
        }
    }
}

/// Prepare a reader for HTTP input
fn handle_http_input(args: &ProgramArgs) -> Result<Reader, Error> {
    let url = args.input.http_input.as_ref().unwrap();  // is_some checked in parent function
    let update_rate;
    if args.http_input.http_input_rate.is_some() {        
        update_rate = args.http_input.http_input_rate.unwrap();
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
fn handle_https_input(args: &ProgramArgs) -> Result<Reader, Error> {
    let url = args.input.https_input.as_ref().unwrap();  // is_some checked in parent function
    let update_rate;
    if args.https_input.https_input_rate.is_some() {        
        update_rate = args.https_input.https_input_rate.unwrap();
        info!("Using HTTPS input rate of {} milliseconds", update_rate);
    } else {        
        update_rate = HttpsReader::DEFAULT_UPDATE_RATE;
        info!("Using default HTTPS input rate of {} milliseconds", update_rate);
    }
    // TODO:  Other HTTPS parameters here!
    let https_reader = HttpsReader::new(url, update_rate)?;
    info!("Using HTTPS input");
    Ok(Reader::Https(https_reader))
}

/// Prepare a reader for STDIN input
fn handle_stdin_input() -> Reader {
    info!("Using STDIN input");
    Reader::Stdin(StdinReader::new())
}

/// Prepare a reader for UDP input
async fn handle_udp_input(args: &ProgramArgs) -> Result<Reader, Error> {
    let address = args.input.udp_input.as_ref().unwrap();  // is_some checked in parent function
    match UdpReader::new(address).await {
        Ok(udp_reader) => {            
            info!("Using UDP input");
            Ok(Reader::Udp(udp_reader))
        }
        Err(error) => {
            let error_message =
                format!("Cannot open input UDP address {:?}: {}", &address, error);
            error!("{}", error_message);
            Err(Error::new(error.kind(), error_message))
        }
    }
}


/// Select the wanted input implementation from the command line args
pub async fn get_input_reader(args: &ProgramArgs) -> Result<Reader, Error> {    
    let mut maybe_reader: Option<Reader> = None;
    if args.input.file_input.is_some() {
        check_reader_set(&maybe_reader)?;
        maybe_reader = Some(handle_file_input(&args).await?);        
    } 
    if args.input.http_input.is_some() {
        check_reader_set(&maybe_reader)?;
        maybe_reader = Some(handle_http_input(&args)?)        
    }
    if args.input.https_input.is_some() {
        check_reader_set(&maybe_reader)?;
        maybe_reader = Some(handle_https_input(&args)?)        
    }
    if args.input.stdin_input {
        check_reader_set(&maybe_reader)?;
        maybe_reader = Some(handle_stdin_input());        
    } 
    
    if args.input.udp_input.is_some() {
        check_reader_set(&maybe_reader)?;
        maybe_reader = Some(handle_udp_input(&args).await?);        
    }
    
    match maybe_reader {
        Some(reader) => Ok(reader),
        None => {
            let error_message = "No input source provided!";
            error!("{}", error_message);
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
        }
    }    
}


pub async fn get_output_writers(args: &ProgramArgs) -> Result<Vec<Writer>, Error> {
    let mut writers: Vec<Writer> = Vec::new();
    if args.output.stdout_output {
        writers.push(Writer::Stdout(StdoutWriter::new()));
        info!("Using STDOUT output");
    }
    if args.output.file_output.is_some() {
        let file_paths = args.output.file_output.as_ref().unwrap();
        for file_path in file_paths {
            match FileWriter::new(file_path).await {
                Ok(file_writer) => {
                    writers.push(Writer::File(file_writer));
                    info!("Using FILE output");
                }
                Err(error) => {
                    let error_message = format!("File output error {:?}: {}", file_path, error);
                    error!("{}", error_message);
                    return Err(Error::new(error.kind(), error_message));
                }
            }
        }
    }
    if args.output.udp_output.is_some() {
        let addresses = args.output.udp_output.as_ref().unwrap();
        for address in addresses {
            match UdpWriter::new(address).await {
                Ok(udp_writer) => {
                    writers.push(Writer::Udp(udp_writer));
                    info!("Using UDP output");
                }
                Err(error) => {
                    let error_message = format!("UDP output error {}: {}", address, error);
                    error!("{}", error_message);
                    return Err(Error::new(error.kind(), error_message));
                }
            }
        }
    } 
     if args.output.http_output.is_some() {
        let urls = args.output.http_output.as_ref().unwrap();
        for url in urls {            
            let output_rate: Duration;
            let delimiter: Vec<u8>;
            let include_delimiter: bool;
            if args.http_output.http_output_delimiter.is_some() {
                delimiter = args.http_output.http_output_delimiter.as_ref().unwrap().to_vec();
            } else {
                delimiter = "\n".as_bytes().to_vec();
            }
            if args.http_output.http_output_include_delimiter.is_some() {
                include_delimiter = args.http_output.http_output_include_delimiter.unwrap();
            } else {
                include_delimiter = false;
            }
            if args.http_output.http_output_rate.is_some() {
                output_rate = Duration::from_millis(args.http_output.http_output_rate.unwrap());
            } else {
                output_rate = Duration::from_millis(5000);
            }
            match HttpWriter::new(&url, delimiter, include_delimiter, output_rate) {            
                Ok(http_writer) => {
                    writers.push(Writer::Http(http_writer));
                    info!("Using HTTP output");
                }
                Err(error) => {
                    let error_message = format!("HTTP URL error {}: {}", &url, error);
                    error!("{}", error_message);
                    return Err(Error::new(error.kind(), error_message));
                }
            }
        }
    }
    if args.output.tls_output.is_some() {
        let addresses = args.output.tls_output.as_ref().unwrap();
        for address in addresses {
            match get_tls_config(args) {
                Ok(tls_config) => match TlsReaderWriter::new(address.to_owned(), tls_config).await {
                    Ok(tls_writer) => {
                        writers.push(Writer::Tls(tls_writer));
                        info!("Using TLS output");
                    }
                    Err(error) => {
                        let error_message = format!("TLS output error {}: {}", &address, error);
                        error!("{}", error_message);
                        return Err(Error::new(error.kind(), error_message));
                    }
                },
                Err(error) => {
                    let error_message = format!("TLS setup error: {}", error);
                    error!("{}", error_message);
                    return Err(Error::new(error.kind(), error_message));
                }
            }
        }
    }
    if writers.is_empty() {
        let error = "No output destination provided!";
        error!("{}", error);
        Err(Error::new(io::ErrorKind::InvalidInput, error))
    } else {
        Ok(writers)
    }
}

fn get_tls_config(args: &ProgramArgs) -> Result<ClientConfig, std::io::Error> {
    // setup root cert store
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(TLS_SERVER_ROOTS.iter().cloned());
    if args.tls_output.tls_output_root_ca.is_some() {
        match get_root_ca(
            args.tls_output.tls_output_root_ca.as_ref().unwrap(),
            &mut root_cert_store,
        ) {
            Ok(()) => {} // no issues loading CA roots
            Err(error) => {
                let error_message =
                    format!("Error loading certificate authority (CA) roots: {}", error);
                error!("{}", error_message);
                return Err(Error::new(ErrorKind::Other, error_message));
            }
        }
    }
    // begin build the config
    // check if no verification was requested
    let config: ConfigBuilder<ClientConfig, WantsClientCert> =
        if args.tls_output.tls_output_skip_server_verify {
            let dangerous_config = ConfigBuilder::dangerous(ClientConfig::builder());
            dangerous_config
                .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new()))
        } else {
            ClientConfig::builder().with_root_certificates(root_cert_store)
        };
    // see if client auth is needed
    match args.tls_output.tls_output_cert_chain.as_ref() {
        Some(tls_cert_chain_path) => {
            // get certificate chain
            match get_tls_cert_chain(tls_cert_chain_path) {
                Ok(cert_chain) => {
                    // get client certificate
                    match args.tls_output.tls_output_client_key.as_ref() {
                        Some(tls_client_key_path) => {
                            match get_tls_client_key(tls_client_key_path) {
                                Ok(client_key) => {
                                    // finish building the config with cert chain and client key
                                    match config.with_client_auth_cert(cert_chain, client_key) {
                                        Ok(tls_config) => Ok(tls_config),
                                        Err(error) => {
                                            let error_message = format!("Error creating TLS config with cert chain and client key: {}", error);
                                            error!("{}", error_message);
                                            Err(Error::new(ErrorKind::Other, error_message))
                                        }
                                    }
                                }
                                Err(error) => {
                                    // failed to get client key
                                    let error_message = format!(
                                        "Error getting client key {:?}: {}",
                                        tls_client_key_path, error
                                    );
                                    error!("{}", error_message);
                                    Err(Error::new(ErrorKind::Other, error_message))
                                }
                            }
                        }
                        None => {
                            // the user should have provided a client key too
                            let error_message = "Certificate chain (--tls-cert-chain) requires client key (--tls-client-key) to also be used";
                            error!("{}", error_message);
                            Err(Error::new(ErrorKind::Other, error_message))
                        }
                    }
                }
                Err(error) => {
                    // failed to get cert chain
                    let error_message = format!(
                        "Error getting certificate chain {:?}: {}",
                        tls_cert_chain_path, error
                    );
                    error!("{}", error_message);
                    Err(Error::new(ErrorKind::Other, error_message))
                }
            }
        }
        None => {
            // make sure the user did not give a client certificate too
            if args.tls_output.tls_output_client_key.is_some() {
                // error: a cert chain is needed if the user is providing a client key
                let error_message = "Client key (--tls-client-key) requires certificate chain (--tls-cert-chain) to also be used";
                error!("{}", error_message);
                return Err(Error::new(ErrorKind::Other, error_message));
            }
            // finishing build the config with no client authentication
            Ok(config.with_no_client_auth())
        }
    }
}


// helper functions for TLS certificates and keys
fn get_root_ca(
    tls_root_ca_path: &PathBuf,
    root_cert_store: &mut RootCertStore,
) -> Result<(), std::io::Error> {
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
                                let error_message = format!("Error adding certificate authority (CA) to root cert store: {}", error);
                                error!("{}", error_message);
                                return Err(Error::new(ErrorKind::Other, error_message));
                            }
                        }
                    }
                    Err(error) => {
                        let error_message = format!(
                            "Error parsing certificate authority (CA) from {:?}: {}",
                            &tls_root_ca_path, error
                        );
                        error!("{}", error_message);
                        return Err(Error::new(ErrorKind::Other, error_message));
                    }
                }
            }
        }
        Err(error) => {
            let error_message = format!(
                "Cannot open TLS root CA file: {:?}: {}",
                &tls_root_ca_path, error
            );
            error!("{}", error_message);
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
        }
    }
    Ok(())
}

fn get_tls_cert_chain(
    tls_cert_chain_path: &PathBuf,
) -> Result<Vec<CertificateDer<'static>>, std::io::Error> {
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
                            format!("Error adding certificate to certificate chain: {}", error);
                        error!("{}", error_message);
                        return Err(Error::new(ErrorKind::Other, error_message));
                    }
                }
            }
        }
        Err(error) => {
            let error_message = format!(
                "Cannot open TLS certificate chain file: {:?}: {}",
                &tls_cert_chain_path, error
            );
            error!("{}", error_message);
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
        }
    }
    Ok(cert_chain)
}

fn get_tls_client_key(
    tls_client_key_path: &PathBuf,
) -> Result<PrivateKeyDer<'static>, std::io::Error> {
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
                        error!("{}", error_message);
                        return Err(Error::new(ErrorKind::InvalidInput, error_message));
                    }
                },
                Err(error) => {
                    let error_message = format!(
                        "Invalid or corrupted TLS client key file: {:?}: {}",
                        &tls_client_key_path, error
                    );
                    error!("{}", error_message);
                    return Err(Error::new(ErrorKind::InvalidInput, error_message));
                }
            }
        }
        Err(error) => {
            let error_message = format!(
                "Cannot open TLS client key file: {:?}: {}",
                &tls_client_key_path, error
            );
            error!("{}", error_message);
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
        }
    }
    Ok(private_key_der)
}

// to allow for --tls-skip-server-verify to work
// NOTE:  this is dangerous to use and should not be used in production
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


impl ProgramArgs {

    pub async fn to_parameters(&self) -> Result<Parameters, std::io::Error> {
        /*
        // parse input UDP address if present
        if args.input.udp_input.is_some()
            && !good_tcp_udp_address(args.input.udp_input.clone().unwrap().as_str())
        {
            eprintln!(
                "Invalid UDP input address: {}",
                args.input.udp_input.unwrap()
            );
            return ExitCode::FAILURE;
        }
        // parse HTTP input URL if present
        if args.input.http_input.is_some() && !good_url(args.input.http_input.clone().unwrap().as_str())
        {
            eprintln!("Invalid HTTP input URL: {}", args.input.http_input.unwrap());
            return ExitCode::FAILURE;
        }
        // parse output UDP address if present
        if args.output.udp_output.is_some() {
            let addresses = args.output.udp_output.clone().unwrap();
            for address in addresses {
                if !good_tcp_udp_address(&address) {
                    eprintln!("Invalid UDP output address: {}", address);
                    return ExitCode::FAILURE;
                }
            }
        }
        // parse HTTP output URL if present
        if args.output.http_output.is_some()
            && !good_url(args.output.http_output.clone().unwrap().as_str())
        {
            eprintln!(
                "Invalid HTTP output URL: {}",
                args.output.http_output.unwrap()
            );
            return ExitCode::FAILURE;
        }
        // parse TLS (TCP) address if present
        if args.output.tls_output.is_some() {
            let addresses = args.output.tls_output.clone().unwrap();
            for address in addresses {
                if !good_tcp_udp_address(&address) {
                    eprintln!("Invalid TLS output address: {}", address);
                    return ExitCode::FAILURE;
                }
            }
        }
        */

        Ok(Parameters::default())
    }
}

