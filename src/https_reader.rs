use crate::datapipe_types::{InputReader, good_url};
/// Reader for HTTPS
use bytes::Bytes;
use log::{error, trace};
use reqwest::{Certificate, Identity, StatusCode, tls::CertificateRevocationList};
use std::io::{Error, ErrorKind};
use std::time::Duration;

/// Reader that polls an HTTPS endpoint on a fixed interval.
#[derive(Debug)]
pub struct HttpsReader {
    client: reqwest::Client,
    url: url::Url,
    read_interval: tokio::time::Interval,
}

impl HttpsReader {
    /// Default HTTPS polling interval.
    pub const DEFAULT_READ_RATE: Duration = Duration::from_secs(5);

    /// Creates a new [`HttpsReader`] with the provided TLS verification and client identity options.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::https_reader::HttpsReader;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), std::io::Error> {
    /// let reader = HttpsReader::new(
    ///     "https://127.0.0.1:8443/data",
    ///     Duration::from_secs(1),
    ///     None,
    ///     None,
    ///     None,
    ///     false,
    ///     true,
    /// )?;
    /// # drop(reader);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the URL is invalid or the HTTPS client cannot be configured.
    pub fn new(
        https_input_url: &str,
        read_rate: Duration,
        maybe_root_certs: Option<Vec<Certificate>>,
        maybe_crls: Option<Vec<CertificateRevocationList>>,
        maybe_identity: Option<Identity>,
        allow_invalid_hostnames: bool,
        allow_invalid_certs: bool,
    ) -> Result<Self, Error> {
        // HTTP client init and configuration
        let url = good_url(https_input_url, "https://")?;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let mut client_builder = reqwest::Client::builder().user_agent("datapipe");
        let needs_certs_only =
            maybe_crls.is_some() || (allow_invalid_hostnames && !allow_invalid_certs);
        match (maybe_root_certs, needs_certs_only) {
            (Some(root_certs), true) => {
                client_builder = client_builder.tls_certs_only(root_certs);
            }
            (Some(root_certs), false) => {
                client_builder = client_builder.tls_certs_merge(root_certs);
            }
            (None, true) => {
                client_builder = client_builder.tls_certs_only(std::iter::empty());
            }
            (None, false) => {}
        }
        if let Some(crls) = maybe_crls {
            client_builder = client_builder.tls_crls_only(crls);
        }
        if let Some(identity) = maybe_identity {
            client_builder = client_builder.identity(identity);
        }
        if allow_invalid_hostnames {
            client_builder = client_builder.tls_danger_accept_invalid_hostnames(true);
        }
        if allow_invalid_certs {
            client_builder = client_builder.tls_danger_accept_invalid_certs(true);
        }
        match client_builder.build() {
            Ok(client) => Ok(Self {
                client,
                url,
                read_interval: tokio::time::interval(read_rate),
            }),
            Err(error) => {
                let error_message = format!("Error configuring HTTPS Input client: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::InvalidInput, error_message))
            }
        }
    }
}

impl InputReader for HttpsReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        self.read_interval.tick().await; // wait until it is time to read
        match self.client.get(self.url.as_str()).send().await {
            Ok(response) => {
                trace!("HttpsInput:  Web server response is: {:?}", response);
                match response.status() {
                    StatusCode::OK => {
                        trace!("HttpsInput:  Status is OK. Getting response body bytes");
                        match response.bytes().await {
                            Ok(bytes) => Ok(bytes),
                            Err(error) => {
                                let error_message = format!(
                                    "HttpsInput:  Error converting response body to bytes: {}",
                                    error
                                );
                                error!("{}", error_message);
                                Err(Error::other(error_message))
                            }
                        }
                    }
                    _ => {
                        error!("HttpsInput:  non-Ok status from web server: {:?}", response);
                        match response.error_for_status() {
                            Ok(res) => {
                                let error_message = format!(
                                    "HttpsInput:  Failed converting web server response to error: {:?}",
                                    res
                                );
                                error!("{}", error_message);
                                Err(Error::other(error_message))
                            }
                            Err(error) => {
                                let error_message =
                                    format!("HttpsInput:  decoded web server status: {}", error);
                                error!("{}", error_message);
                                Err(Error::other(error_message))
                            }
                        }
                    }
                }
            }
            Err(error) => {
                let error_message = format!("HttpsInput:  Error getting HTTP input: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::NotConnected, error_message))
            }
        }
    }
}
