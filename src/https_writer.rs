/// Writer for HTTPS
use crate::datapipe_types::{OutputWriter, good_url};
use crate::http_writer::DelimitedHttpSender;
use crate::https_reader::build_https_client;
use reqwest::{Certificate, Identity, tls::CertificateRevocationList};
use std::io::Error;
use std::time::Duration;

/// Writer that sends delimited chunks securely to an HTTPS endpoint.
#[derive(Debug)]
pub struct HttpsWriter {
    sender: DelimitedHttpSender,
}

impl HttpsWriter {
    /// Default payload delimiter (`\n`).
    pub const DEFAULT_DELIMITER: [u8; 1] = *b"\n";
    /// Default minimum interval between HTTPS writes.
    pub const DEFAULT_WRITE_RATE: Duration = Duration::from_secs(5);

    /// Creates a new [`HttpsWriter`] with the provided TLS verification and client identity options.
    ///
    /// A `write_rate` of [`Duration::ZERO`] sends delimited payloads immediately without batching.
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
    /// Returns [`std::io::Error`] if the URL is invalid, the delimiter is empty, or the HTTPS client cannot be configured.
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
        let url = good_url(https_output_url, "https://")?;
        let client = build_https_client(
            "HTTPS Output",
            maybe_root_certs,
            maybe_crls,
            maybe_identity,
            allow_invalid_hostnames,
            allow_invalid_certs,
        )?;
        let sender = DelimitedHttpSender::new(
            client,
            url,
            "HttpsOutput",
            https_output_delimiter,
            https_output_include_delimiter,
            write_rate,
        )?;
        Ok(Self { sender })
    }
}

impl OutputWriter for HttpsWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.sender.write(bytes).await
    }

    async fn close(&mut self) -> Result<(), Error> {
        self.sender.close().await
    }
}
