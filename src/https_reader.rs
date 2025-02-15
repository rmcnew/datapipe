/// Reader for HTTPS

use bytes::Bytes;
use crate::datapipe_types::{good_url, InputReader};
use log::{error, trace};
use reqwest::{Certificate, tls::CertificateRevocationList, Identity, StatusCode};
use std::io::{Error, ErrorKind};

#[derive(Debug)]
pub struct HttpsReader {
    client: reqwest::Client,
    url: url::Url,    
    update_interval: tokio::time::Interval,
}


impl HttpsReader {
    pub const DEFAULT_UPDATE_RATE: u64 = 5000;  // milliseconds

    pub fn new(https_input_url: &str, update_rate: u64, maybe_root_cert: Option<Certificate>, maybe_crls: Option<Vec<CertificateRevocationList>>, maybe_identity: Option<Identity>, accept_invalid_hostnames: bool, accept_invalid_certs: bool) -> Result<Self, Error> {
        // HTTP client init and configuration
        let url = good_url(https_input_url, "https://")?;
        let mut client_builder = reqwest::Client::builder()
            .user_agent("datapipe")
            .tls_built_in_root_certs(true)  // enable system root certs
            .tls_built_in_webpki_certs(true);  // enable webpki root certs
        if maybe_root_cert.is_some() {
            client_builder = client_builder.add_root_certificate(maybe_root_cert.unwrap());
        }
        if maybe_crls.is_some() {
            client_builder = client_builder.add_crls(maybe_crls.unwrap());
        }
        if maybe_identity.is_some() {
            client_builder = client_builder.identity(maybe_identity.unwrap());
        }
        if accept_invalid_hostnames {
            client_builder = client_builder.danger_accept_invalid_hostnames(true);
        }
        if accept_invalid_certs {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }
        match client_builder.build() {
            Ok(client) => {
                Ok(Self {
                    client,
                    url,
                    update_interval: tokio::time::interval(std::time::Duration::from_millis(update_rate)),
                })
            }
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
        self.update_interval.tick().await;  // wait until it is time to read
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
                                Err(Error::new(ErrorKind::Other, error_message))
                            }
                        }
                    }
                    _ => {
                        error!("HttpsInput:  non-Ok status from web server: {:?}", response);
                        match response.error_for_status() {
                            Ok(res) => {
                                let error_message = format!("HttpsInput:  Failed converting web server response to error: {:?}", res);
                                error!("{}", error_message);
                                Err(Error::new(ErrorKind::Other, error_message))
                            }
                            Err(error) => {
                                let error_message = format!("HttpsInput:  decoded web server status: {}", error);
                                error!("{}", error_message);
                                Err(Error::new(ErrorKind::Other, error_message))
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
