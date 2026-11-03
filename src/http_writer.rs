/// Writer for HTTP
use crate::datapipe_types::{OutputWriter, good_url};
use log::{error, trace};
use std::io::{Error, ErrorKind};
use std::time::{Duration, Instant};

/// Writer that sends delimited chunks to an HTTP endpoint.
#[derive(Debug)]
pub struct HttpWriter {
    client: reqwest::Client,
    url: url::Url,
    delimiter: Vec<u8>,
    include_delimiter: bool,
    output_rate: std::time::Duration,
    last_output: Instant,
    payload: Vec<u8>,    // the data to be sent
    buffer: Vec<u8>,     // data received for which we have not yet seen the delimiter
    buffer_index: usize, // the current position in the buffer that we have scanned to
}

impl HttpWriter {
    /// Default payload delimiter (`\n`).
    pub const DEFAULT_DELIMITER: [u8; 1] = *b"\n";
    /// Default minimum interval between HTTP writes.
    pub const DEFAULT_WRITE_RATE: Duration = Duration::from_secs(5);

    /// Creates a new [`HttpWriter`] targeting `http_output_url`.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::http_writer::HttpWriter;
    /// use std::time::Duration;
    ///
    /// # fn main() -> Result<(), std::io::Error> {
    /// let writer = HttpWriter::new(
    ///     "http://127.0.0.1:8080/ingest",
    ///     HttpWriter::DEFAULT_DELIMITER.to_vec(),
    ///     true,
    ///     Duration::from_millis(500),
    /// )?;
    /// # drop(writer);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the URL is invalid or the HTTP client cannot be built.
    pub fn new(
        http_output_url: &str,
        http_output_delimiter: Vec<u8>,
        http_output_include_delimiter: bool,
        http_output_rate: Duration,
    ) -> Result<Self, Error> {
        // HTTP client init and configuration
        let url = good_url(http_output_url, "http://")?;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .user_agent("datapipe")
            .build()
            .map_err(|error| {
                let error_message = format!("Error configuring HTTP Output client: {error}");
                error!("{error_message}");
                Error::new(ErrorKind::InvalidInput, error_message)
            })?;
        Ok(Self {
            client,
            url,
            delimiter: http_output_delimiter,
            include_delimiter: http_output_include_delimiter,
            output_rate: http_output_rate,
            last_output: Instant::now() - http_output_rate, // backdate so we can write immediately after initialization
            payload: Vec::new(),
            buffer: Vec::new(),
            buffer_index: 0,
        })
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
                trace!("HttpOutput:  Web server response is: {:?}", response);
                // update last_output to now()
                self.last_output = Instant::now();
                Ok(())
            }
            Err(error) => {
                let error_message = format!("HttpOutput:  Error sending HTTP output: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::NotConnected, error_message))
            }
        }
    }
}

impl OutputWriter for HttpWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        // pushed received bytes onto buffer
        self.buffer.append(&mut bytes.to_vec());
        let found_delimiter = self.scan_for_delimiter();
        if found_delimiter {
            self.extract_payload();
        }
        // if last_output was older than output_rate and there is data to send
        if self.last_output.elapsed() >= self.output_rate && !self.payload.is_empty() {
            self.send_payload().await?;
        }
        Ok(())
    }
}
