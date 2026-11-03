/// Writer for HTTPS
use crate::datapipe_types::{OutputWriter, good_url};
use log::{error, trace};
use reqwest::{Certificate, Identity, tls::CertificateRevocationList};
use std::io::{Error, ErrorKind};
use std::time::Duration;

/// Writer that sends delimited chunks securely to an HTTPS endpoint.
#[derive(Debug)]
pub struct HttpsWriter {
    client: reqwest::Client,
    url: url::Url,
    delimiter: Vec<u8>,
    include_delimiter: bool,
    write_interval: tokio::time::Interval,
    payload: Vec<u8>,    // the data to be sent
    buffer: Vec<u8>,     // data received for which we have not yet seen the delimiter
    buffer_index: usize, // the current position in the buffer that we have scanned to
}

impl HttpsWriter {
    /// Default payload delimiter (`\n`).
    pub const DEFAULT_DELIMITER: [u8; 1] = *b"\n";
    /// Default minimum interval between HTTPS writes.
    pub const DEFAULT_WRITE_RATE: Duration = Duration::from_secs(5);

    /// Creates a new [`HttpsWriter`] with the provided TLS verification and client identity options.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::https_writer::HttpsWriter;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), std::io::Error> {
    /// let writer = HttpsWriter::new(
    ///     "https://127.0.0.1:8443/ingest",
    ///     HttpsWriter::DEFAULT_DELIMITER.to_vec(),
    ///     true,
    ///     Duration::from_secs(1),
    ///     None,
    ///     None,
    ///     None,
    ///     false,
    ///     true,
    /// )?;
    /// # drop(writer);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the URL is invalid or the HTTPS client cannot be configured.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        https_output_url: &str,
        https_output_delimiter: Vec<u8>,
        https_output_include_delimiter: bool,
        write_rate: Duration,
        maybe_root_certs: Option<Vec<Certificate>>,
        maybe_crls: Option<Vec<CertificateRevocationList>>,
        maybe_identity: Option<Identity>,
        allow_invalid_hostnames: bool,
        allow_invalid_certs: bool,
    ) -> Result<Self, Error> {
        // HTTP client init and configuration
        let url = good_url(https_output_url, "https://")?;
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
                delimiter: https_output_delimiter,
                include_delimiter: https_output_include_delimiter,
                write_interval: tokio::time::interval(write_rate),
                payload: Vec::new(),
                buffer: Vec::new(),
                buffer_index: 0,
            }),
            Err(error) => {
                let error_message = format!("Error configuring HTTPS Output client: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::InvalidInput, error_message))
            }
        }
    }

    fn scan_for_delimiter(&mut self) -> bool {
        // scan through buffer looking for the delimiter sequence
        let mut found_delimiter = false;
        for maybe_delimiter in self.buffer[self.buffer_index..].windows(self.delimiter.len()) {
            if maybe_delimiter == self.delimiter.as_slice() {
                // break out of loop
                found_delimiter = true;
                break;
            }
            self.buffer_index += 1; // buffer_index should be pointing to the beginning of the delimiter sequence if it is found
        }
        found_delimiter
    }

    fn extract_payload(&mut self) {
        // advance the index past the delimiter
        self.buffer_index += self.delimiter.len();
        // split the buffer
        let mut rest = self.buffer.split_off(self.buffer_index);
        if !self.include_delimiter {
            // if we are not including the delimiter, truncate it from the buffer
            self.buffer
                .truncate(self.buffer.len() - self.delimiter.len());
        }
        // swap the buffer with rest, so the buffer has the rest
        std::mem::swap(&mut rest, &mut self.buffer);
        // append the bytes from the buffer (and possibly the delimiter) to payload for sending
        self.payload.append(&mut rest);
    }

    async fn send_payload(&mut self) -> Result<(), Error> {
        // grab payload and replace it with an empty one
        let payload = std::mem::take(&mut self.payload);
        // send payload
        match self
            .client
            .put(self.url.as_str())
            .body(payload)
            .send()
            .await
        {
            Ok(response) => {
                trace!("HttpsOutput:  Web server response is: {:?}", response);
                Ok(())
            }
            Err(error) => {
                let error_message = format!("HttpsOutput:  Error sending HTTPS output: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::NotConnected, error_message))
            }
        }
    }
}

impl OutputWriter for HttpsWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        // pushed received bytes onto buffer
        self.buffer.append(&mut bytes.to_vec());
        let found_delimiter = self.scan_for_delimiter();
        if found_delimiter {
            self.extract_payload();
        }
        self.write_interval.tick().await; // wait until it is time to read
        // if last_output was older than output_rate and there is data to send
        if !self.payload.is_empty() {
            self.send_payload().await?;
        }
        Ok(())
    }
}
