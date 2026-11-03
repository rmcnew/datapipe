use crate::datapipe_types::{InputReader, good_url};
/// Reader for HTTP
use bytes::Bytes;
use log::{error, trace};
use reqwest::StatusCode;
use std::io::{Error, ErrorKind};

/// Reader that polls an HTTP endpoint on a fixed interval.
#[derive(Debug)]
pub struct HttpReader {
    client: reqwest::Client,
    url: url::Url,
    update_interval: tokio::time::Interval,
}

impl HttpReader {
    /// Default HTTP polling interval in milliseconds.
    pub const DEFAULT_UPDATE_RATE: u64 = 5000; // milliseconds

    /// Creates a new [`HttpReader`] polling `http_input_url` every `update_rate` milliseconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use datapipe::http_reader::HttpReader;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() -> Result<(), std::io::Error> {
    /// let reader = HttpReader::new("http://127.0.0.1:8080/data", 1000)?;
    /// # drop(reader);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the URL is invalid or the HTTP client cannot be built.
    pub fn new(http_input_url: &str, update_rate: u64) -> Result<Self, Error> {
        // HTTP client init and configuration
        let url = good_url(http_input_url, "http://")?;
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::builder()
            .user_agent("datapipe")
            .build()
            .map_err(|error| {
                let error_message = format!("Error configuring HTTP Input client: {error}");
                error!("{error_message}");
                Error::new(ErrorKind::InvalidInput, error_message)
            })?;
        Ok(Self {
            client,
            url,
            update_interval: tokio::time::interval(std::time::Duration::from_millis(update_rate)),
        })
    }
}

impl InputReader for HttpReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        self.update_interval.tick().await; // wait until it is time to read
        match self.client.get(self.url.as_str()).send().await {
            Ok(response) => {
                trace!("HttpInput:  Web server response is: {:?}", response);
                match response.status() {
                    StatusCode::OK => {
                        trace!("HttpInput:  Status is OK. Getting response body bytes");
                        match response.bytes().await {
                            Ok(bytes) => Ok(bytes),
                            Err(error) => {
                                let error_message = format!(
                                    "HttpInput:  Error converting response body to bytes: {}",
                                    error
                                );
                                error!("{}", error_message);
                                Err(Error::other(error_message))
                            }
                        }
                    }
                    _ => {
                        error!("HttpInput:  non-Ok status from web server: {:?}", response);
                        match response.error_for_status() {
                            Ok(res) => {
                                let error_message = format!(
                                    "HttpInput:  Failed converting web server response to error: {:?}",
                                    res
                                );
                                error!("{}", error_message);
                                Err(Error::other(error_message))
                            }
                            Err(error) => {
                                let error_message =
                                    format!("HttpInput:  decoded web server status: {}", error);
                                error!("{}", error_message);
                                Err(Error::other(error_message))
                            }
                        }
                    }
                }
            }
            Err(error) => {
                let error_message = format!("HttpInput:  Error getting HTTP input: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::NotConnected, error_message))
            }
        }
    }
}
