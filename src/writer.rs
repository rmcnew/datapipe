use crate::args::ProgramArgs;
use crate::datapipe_types::OutputWriter;
use crate::file_writer::FileWriter;
use crate::http_writer::HttpWriter;
use crate::stdout_writer::StdoutWriter;
use crate::tls_writer::TlsWriter;
use crate::udp_writer::UdpWriter;
use log::{error, info};
use rustls_pemfile::{certs, private_key};
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::path::PathBuf;
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


// Combined writer
pub enum Writer {
    Stdout(StdoutWriter),
    File(FileWriter),
    Udp(UdpWriter),
    Http(HttpWriter),
    Tls(TlsWriter),
}

impl OutputWriter for Writer {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match self {
            Self::Stdout(stdout_writer) => stdout_writer.write(bytes).await,
            Self::File(file_writer) => file_writer.write(bytes).await,
            Self::Udp(udp_writer) => udp_writer.write(bytes).await,
            Self::Http(http_writer) => http_writer.write(bytes).await,
            Self::Tls(tls_writer) => tls_writer.write(bytes).await,
        }
    }
}

impl std::default::Default for Writer {
    fn default() -> Self {
        Self::Stdout(StdoutWriter::new())
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
            let url = args.output.http_output.as_ref().unwrap();
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
    if args.output.tls_output.is_some() {
        let addresses = args.output.tls_output.as_ref().unwrap();
        for address in addresses {
            match get_tls_config(args) {
                Ok(tls_config) => match TlsWriter::new(address.to_owned(), tls_config).await {
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
    if args.tls_output.tls_root_ca.is_some() {
        match get_root_ca(
            args.tls_output.tls_root_ca.as_ref().unwrap(),
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
        if args.tls_output.tls_skip_server_verify.is_some()
            && args.tls_output.tls_skip_server_verify.unwrap()
        {
            let dangerous_config = ConfigBuilder::dangerous(ClientConfig::builder());
            dangerous_config
                .with_custom_certificate_verifier(Arc::new(NoCertificateVerification::new()))
        } else {
            ClientConfig::builder().with_root_certificates(root_cert_store)
        };
    // see if client auth is needed
    match args.tls_output.tls_cert_chain.as_ref() {
        Some(tls_cert_chain_path) => {
            // get certificate chain
            match get_tls_cert_chain(tls_cert_chain_path) {
                Ok(cert_chain) => {
                    // get client certificate
                    match args.tls_output.tls_client_key.as_ref() {
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
            if args.tls_output.tls_client_key.is_some() {
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


