/// Writer for HTTP
use crate::datapipe_types::{OutputWriter, good_url};
use log::{error, trace};
use std::io::{Error, ErrorKind};
use std::time::{Duration, Instant};

/// Shared delimited chunk buffer and rate-limited HTTP/HTTPS PUT sender.
#[derive(Debug)]
pub(crate) struct DelimitedHttpSender {
    client: reqwest::Client,
    url: url::Url,
    label: &'static str,
    delimiter: Vec<u8>,
    include_delimiter: bool,
    output_rate: Duration,
    last_output: Option<Instant>,
    payload: Vec<u8>,    // complete delimited payloads queued to send
    buffer: Vec<u8>,     // trailing bytes received before seeing the delimiter
    buffer_index: usize, // scan offset within `buffer`
}

impl DelimitedHttpSender {
    pub(crate) fn new(
        client: reqwest::Client,
        url: url::Url,
        label: &'static str,
        delimiter: Vec<u8>,
        include_delimiter: bool,
        output_rate: Duration,
    ) -> Result<Self, Error> {
        if delimiter.is_empty() {
            let error_message = format!("{label}: HTTP output delimiter cannot be empty");
            error!("{error_message}");
            return Err(Error::new(ErrorKind::InvalidInput, error_message));
        }
        Ok(Self {
            client,
            url,
            label,
            delimiter,
            include_delimiter,
            output_rate,
            last_output: None,
            payload: Vec::new(),
            buffer: Vec::new(),
            buffer_index: 0,
        })
    }

    fn ready_to_send(&self) -> bool {
        if self.output_rate.is_zero() {
            return true;
        }
        match self.last_output {
            None => true,
            Some(last) => last.elapsed() >= self.output_rate,
        }
    }

    fn scan_for_delimiter(&mut self) -> bool {
        let Some(slice) = self.buffer.get(self.buffer_index..) else {
            return false;
        };
        for maybe_delimiter in slice.windows(self.delimiter.len()) {
            if maybe_delimiter == self.delimiter.as_slice() {
                return true;
            }
            self.buffer_index += 1;
        }
        false
    }

    fn extract_payload(&mut self) {
        self.buffer_index += self.delimiter.len();
        let mut rest = self.buffer.split_off(self.buffer_index);
        if !self.include_delimiter {
            self.buffer
                .truncate(self.buffer.len() - self.delimiter.len());
        }
        std::mem::swap(&mut rest, &mut self.buffer);
        self.payload.append(&mut rest);
        self.buffer_index = 0;
    }

    fn extract_all_frames(&mut self) {
        while self.scan_for_delimiter() {
            self.extract_payload();
        }
    }

    async fn send_payload(&mut self) -> Result<(), Error> {
        if self.payload.is_empty() {
            return Ok(());
        }
        let payload = std::mem::take(&mut self.payload);
        match self
            .client
            .put(self.url.as_str())
            .body(payload.clone())
            .send()
            .await
        {
            Ok(response) => {
                trace!("{}:  Web server response is: {:?}", self.label, response);
                match response.error_for_status() {
                    Ok(_) => {
                        self.last_output = Some(Instant::now());
                        Ok(())
                    }
                    Err(error) => {
                        self.payload = payload;
                        let error_message =
                            format!("{}:  Web server returned error status: {error}", self.label);
                        error!("{error_message}");
                        Err(Error::other(error_message))
                    }
                }
            }
            Err(error) => {
                self.payload = payload;
                let error_message = format!("{}:  Error sending HTTP output: {error}", self.label);
                error!("{error_message}");
                Err(Error::new(ErrorKind::NotConnected, error_message))
            }
        }
    }

    pub(crate) async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        // Flush already-delimited payloads before mutating buffer so retries never duplicate `bytes`.
        if self.ready_to_send() && !self.payload.is_empty() {
            self.send_payload().await?;
        }
        let rollback = if self.ready_to_send() && self.payload.is_empty() {
            Some((self.buffer.clone(), self.buffer_index))
        } else {
            None
        };
        self.buffer.extend_from_slice(bytes);
        self.extract_all_frames();
        if self.ready_to_send()
            && !self.payload.is_empty()
            && let Err(err) = self.send_payload().await
        {
            if let Some((prev_buffer, prev_index)) = rollback {
                self.buffer = prev_buffer;
                self.buffer_index = prev_index;
                self.payload.clear();
            }
            return Err(err);
        }
        Ok(())
    }

    pub(crate) async fn close(&mut self) -> Result<(), Error> {
        self.extract_all_frames();
        if !self.buffer.is_empty() {
            let mut trailing = std::mem::take(&mut self.buffer);
            self.payload.append(&mut trailing);
            self.buffer_index = 0;
        }
        self.send_payload().await
    }
}

/// Writer that sends delimited chunks to an HTTP endpoint.
#[derive(Debug)]
pub struct HttpWriter {
    sender: DelimitedHttpSender,
}

impl HttpWriter {
    /// Default payload delimiter (`\n`).
    pub const DEFAULT_DELIMITER: [u8; 1] = *b"\n";
    /// Default minimum interval between HTTP writes.
    pub const DEFAULT_WRITE_RATE: Duration = Duration::from_secs(5);

    /// Creates a new [`HttpWriter`] targeting `http_output_url`.
    ///
    /// A `http_output_rate` of [`Duration::ZERO`] sends delimited payloads immediately without batching.
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
    /// Returns [`std::io::Error`] if the URL is invalid, the delimiter is empty, or the HTTP client cannot be built.
    pub fn new(
        http_output_url: &str,
        http_output_delimiter: Vec<u8>,
        http_output_include_delimiter: bool,
        http_output_rate: Duration,
    ) -> Result<Self, Error> {
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
        let sender = DelimitedHttpSender::new(
            client,
            url,
            "HttpOutput",
            http_output_delimiter,
            http_output_include_delimiter,
            http_output_rate,
        )?;
        Ok(Self { sender })
    }
}

impl OutputWriter for HttpWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.sender.write(bytes).await
    }

    async fn close(&mut self) -> Result<(), Error> {
        self.sender.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[test]
    fn test_http_writer_rejects_empty_delimiter() {
        let res = HttpWriter::new(
            "http://127.0.0.1:8080/ingest",
            Vec::new(),
            true,
            Duration::ZERO,
        );
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_http_writer_rate_zero_multiline_and_close_flushes_trailing() {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        let server = tokio::spawn(async move {
            let mut received = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("accept");
                let mut req_buf = Vec::new();
                let mut tmp = [0u8; 4096];
                let header_end = loop {
                    let n = stream.read(&mut tmp).await.expect("read");
                    assert!(n > 0);
                    req_buf.extend_from_slice(&tmp[..n]);
                    if let Some(pos) = req_buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&req_buf[..header_end]);
                let content_len: usize = headers
                    .lines()
                    .find_map(|l| {
                        let (k, v) = l.split_once(':')?;
                        k.eq_ignore_ascii_case("content-length")
                            .then(|| v.trim().parse().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                while req_buf.len() < header_end + content_len {
                    let n = stream.read(&mut tmp).await.expect("read body");
                    assert!(n > 0);
                    req_buf.extend_from_slice(&tmp[..n]);
                }
                received.extend_from_slice(&req_buf[header_end..header_end + content_len]);
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await
                    .expect("write resp");
            }
            received
        });

        let url = format!("http://{addr}/ingest");
        let mut writer =
            HttpWriter::new(&url, b"\r\n".to_vec(), true, Duration::ZERO).expect("writer");
        writer
            .write(b"line1\r\nline2\r\ntrailing")
            .await
            .expect("write");
        writer.close().await.expect("close");

        let body = server.await.expect("server task");
        assert_eq!(body, b"line1\r\nline2\r\ntrailing");
    }
}
