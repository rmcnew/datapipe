use crate::datapipe_types::{InputReader, good_url};
use crate::http_reader::{build_poll_interval, read_http_response};
/// Reader for HTTPS
use bytes::Bytes;
use log::error;
use reqwest::{Certificate, Identity, tls::CertificateRevocationList};
use std::io::{Error, ErrorKind};
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_https_client(
    label: &str,
    maybe_root_certs: Option<Vec<Certificate>>,
    maybe_crls: Option<Vec<CertificateRevocationList>>,
    maybe_identity: Option<Identity>,
    allow_invalid_hostnames: bool,
    allow_invalid_certs: bool,
) -> Result<reqwest::Client, Error> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut client_builder = reqwest::Client::builder().user_agent("datapipe");
    let has_crls = match &maybe_crls {
        Some(crls) if crls.is_empty() => {
            let error_message =
                format!("{label}: Certificate revocation list (CRL) cannot be empty");
            error!("{error_message}");
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
        }
        Some(_) => true,
        None => false,
    };

    let needs_certs_only = has_crls || (allow_invalid_hostnames && !allow_invalid_certs);
    match (maybe_root_certs, needs_certs_only) {
        (Some(root_certs), _) if root_certs.is_empty() => {
            let error_message = format!("{label}: TLS root CA certificate list cannot be empty");
            error!("{error_message}");
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
        }
        (Some(root_certs), true) => {
            client_builder = client_builder.tls_certs_only(root_certs);
        }
        (Some(root_certs), false) => {
            client_builder = client_builder.tls_certs_merge(root_certs);
        }
        (None, true) => {
            let error_message = format!(
                "{label}: Custom root CA certificates are required when using CRLs or allowing invalid hostnames without allowing invalid certificates"
            );
            error!("{error_message}");
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
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

    client_builder.build().map_err(|error| {
        let error_message = format!("Error configuring {label} client: {error}");
        error!("{error_message}");
        Error::new(ErrorKind::InvalidInput, error_message)
    })
}

/// Reader that polls an HTTPS endpoint on a fixed interval.
#[derive(Debug)]
pub struct HttpsReader {
    client: reqwest::Client,
    url: url::Url,
    read_interval: Option<tokio::time::Interval>,
}

impl HttpsReader {
    /// Default HTTPS polling interval.
    pub const DEFAULT_READ_RATE: Duration = Duration::from_secs(5);

    /// Creates a new [`HttpsReader`] with the provided TLS verification and client identity options.
    ///
    /// A `read_rate` of [`Duration::ZERO`] polls continuously without waiting for a timer interval.
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
        let url = good_url(https_input_url, "https://")?;
        let client = build_https_client(
            "HTTPS Input",
            maybe_root_certs,
            maybe_crls,
            maybe_identity,
            allow_invalid_hostnames,
            allow_invalid_certs,
        )?;
        Ok(Self {
            client,
            url,
            read_interval: build_poll_interval(read_rate),
        })
    }
}

impl InputReader for HttpsReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        if let Some(ref mut interval) = self.read_interval {
            interval.tick().await;
        }
        read_http_response(&self.client, &self.url, "HttpsInput").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_https_client_rejects_unsafe_or_empty_tls_inputs() {
        // Empty root CA list must fail
        assert!(build_https_client("test", Some(Vec::new()), None, None, false, false).is_err());
        // Empty CRL list must fail
        assert!(build_https_client("test", None, Some(Vec::new()), None, false, false).is_err());
        // allow_invalid_hostnames without allow_invalid_certs and without custom root CAs must fail
        assert!(build_https_client("test", None, None, None, true, false).is_err());
        // HttpsReader with Duration::ZERO must construct cleanly
        let reader = HttpsReader::new(
            "https://127.0.0.1:8443/data",
            Duration::ZERO,
            None,
            None,
            None,
            false,
            true,
        );
        assert!(reader.is_ok());
    }
}
