/// Reader for HTTP

use bytes::Bytes;
use crate::datapipe_types::InputReader;
use log::{error, trace};
use reqwest::StatusCode;
use std::io::{Error, ErrorKind};

pub struct HttpReader {
    client: reqwest::Client,
    url: url::Url,    
    update_interval: tokio::time::Interval,
}


impl HttpReader {
    pub fn new(http_input_url: &str, update_rate: u64) -> Result<HttpReader, Error> {
        // HTTP client init and configuration
        match url::Url::parse(http_input_url) {
            Ok(url) => {
                Ok(Self {
                    client: reqwest::Client::new(),
                    url,
                    update_interval: tokio::time::interval(std::time::Duration::from_millis(update_rate)),
                })
            }
            Err(error) => {
                let error_message = format!("Error parsing http-input URL: {}", error);
                error!("{}", error_message);
                Err(Error::new(ErrorKind::InvalidInput, error_message))
            }
        }
        
    }
}

impl InputReader for HttpReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        self.update_interval.tick().await;  // wait until it is time to read
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
                                Err(Error::new(ErrorKind::Other, error_message))
                            }
                        }
                    }
                    _ => {
                        error!("HttpInput:  non-Ok status from web server: {:?}", response);
                        match response.error_for_status() {
                            Ok(res) => {
                                let error_message = format!("HttpInput:  Failed converting web server response to error: {:?}", res);
                                error!("{}", error_message);
                                Err(Error::new(ErrorKind::Other, error_message))
                            }
                            Err(error) => {
                                let error_message = format!("HttpInput:  decoded web server status: {}", error);
                                error!("{}", error_message);
                                Err(Error::new(ErrorKind::Other, error_message))
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
