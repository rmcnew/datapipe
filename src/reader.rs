use bytes::{Bytes, BytesMut};
use crate::args::ProgramArgs;
use log::{error, info, trace};
use reqwest::StatusCode;
use std::io::{Error, ErrorKind, SeekFrom};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::net::UdpSocket;

/// Reader trait to simplify reading from various input sources
pub trait InputReader {
    async fn read(&mut self) -> Result<Bytes, Error>;
}

/// Reader for STDIN
pub struct StdinReader {}

impl StdinReader {
    fn new() -> Self {
        Self {}
    }
}

impl InputReader for StdinReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        let mut bytes = BytesMut::new();
        match tokio::io::stdin().read_buf(&mut bytes).await {
            Ok(_bytes_read) => Ok(bytes.freeze()),
            Err(error) => Err(error),
        }
    }
}

/// Reader for files
pub struct FileReader {
    file: tokio::fs::File,
}

impl FileReader {
    async fn new(pathref: &Path) -> Result<Self, Error> {
        Ok(Self {
            file: tokio::fs::OpenOptions::new()
                .read(true)
                .open(pathref)
                .await?,
        })
    }
}

impl InputReader for FileReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        let position = self.file.seek(SeekFrom::Current(0)).await?;
        trace!("Reading input file at position {}", position);
        let mut bytes = BytesMut::new();
        match self.file.read_buf(&mut bytes).await {
            Ok(_bytes_read) => Ok(bytes.freeze()),
            Err(error) => Err(error),
        }
    }
}

/// Reader for UDP
pub struct UdpReader {
    socket: UdpSocket,
}

impl UdpReader {
    async fn new(address: &str) -> Result<UdpReader, Error> {
        trace!("UdpReader listening on {}", address);
        match UdpSocket::bind(address).await {
            Ok(socket) => Ok(UdpReader { socket }),
            Err(error) => Err(error),
        }
    }
}

impl InputReader for UdpReader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        let mut vec_bytes = Vec::with_capacity(512);
        match self.socket.recv_buf_from(&mut vec_bytes).await {
            Ok((_length, _source_address)) => {
                trace!(
                    "UdpReader received {} bytes from {}",
                    _length,
                    _source_address
                );
                Ok(Bytes::from(vec_bytes))
            }
            Err(error) => Err(error),
        }
    }
}

/// Reader for HTTP
pub struct HttpReader {
    client: reqwest::Client,
    url: url::Url,    
    update_interval: tokio::time::Interval,
}


impl HttpReader {
    fn new(http_input_url: &str, update_rate: u64) -> Result<HttpReader, Error> {
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

/// Reader enum holds all input implementations
pub enum Reader {
    Stdin(StdinReader),
    File(FileReader),
    Udp(UdpReader),
    Http(HttpReader),
}

/// Use the input implementation's respective Reader trait
impl InputReader for Reader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        match self {
            Self::Stdin(stdin_reader) => stdin_reader.read().await,
            Self::File(file_reader) => file_reader.read().await,
            Self::Udp(udp_reader) => udp_reader.read().await,
            Self::Http(http_reader) => http_reader.read().await,
        }
    }
}

/// Select the wanted input implementation from the command line args
pub async fn get_input_reader(args: &ProgramArgs) -> Result<Reader, Error> {
    let reader: Reader;
    if args.input.stdin_input {
        reader = Reader::Stdin(StdinReader::new());
        info!("Using STDIN input");
    } else if args.input.file_input.is_some() {
        let file_path = args.input.file_input.as_ref().unwrap();
        match FileReader::new(file_path).await {
            Ok(file_reader) => {
                reader = Reader::File(file_reader);
                info!("Using FILE input");
            }
            Err(error) => {
                let error_message = format!("File input error {:?}: {}", file_path, error);
                error!("{}", error_message);
                return Err(Error::new(error.kind(), error_message));
            }
        }
    } else if args.input.udp_input.is_some() {
        let address = args.input.udp_input.as_ref().unwrap();
        match UdpReader::new(address).await {
            Ok(udp_reader) => {
                reader = Reader::Udp(udp_reader);
                info!("Using UDP input");
            }
            Err(error) => {
                let error_message =
                    format!("Cannot open input UDP address {:?}: {}", &address, error);
                error!("{}", error_message);
                return Err(Error::new(error.kind(), error_message));
            }
        }
    } else if args.input.http_input.is_some() {
        let url = args.input.http_input.as_ref().unwrap();
        let update_rate;
        if args.http_input.http_input_rate.is_some() {
            update_rate = args.http_input.http_input_rate.unwrap();
        } else {
            info!("Using default HTTP input rate of 5 seconds");
            update_rate = 5000;
        }
        match HttpReader::new(url, update_rate) {
            Ok(http_reader) => {
                reader = Reader::Http(http_reader);
                info!("Using HTTP input");
            }
            Err(error) => {
                let error_message = format!("HTTP URL error {}: {}", &url, error);
                error!("{}", error_message);
                return Err(Error::new(error.kind(), error_message));
            }
        }
    } else {
        let error_message = "No input source provided!";
        error!("{}", error_message);
        return Err(Error::new(ErrorKind::InvalidInput, error_message));
    }
    Ok(reader)
}
