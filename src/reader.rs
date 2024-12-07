use bytes::Bytes;
use crate::args::ProgramArgs;
use crate::datapipe_types::InputReader;
use crate::stdin_reader::StdinReader;
use crate::file_reader::FileReader;
use crate::http_reader::HttpReader;
use crate::udp_reader::UdpReader;
use log::{error, info};
use std::io::{Error, ErrorKind};

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

impl std::default::Default for Reader {
    fn default() -> Self {
        Self::Stdin(StdinReader::new())
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
