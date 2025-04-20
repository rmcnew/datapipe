use crate::datapipe_types::InputReader;
use crate::file_reader::FileReader;
use crate::http_reader::HttpReader;
use crate::https_reader::HttpsReader;
use crate::stdin_reader::StdinReader;
use crate::tcp_listen_reader::TcpListenReader;
use crate::tcp_reader_writer::TcpReaderWriter;
use crate::tls_reader_writer::TlsReaderWriter;
use crate::udp_reader::UdpReader;
use bytes::Bytes;
use std::io::Error;

/// Reader enum holds all input implementations
#[derive(Debug)]
pub enum Reader {
    File(FileReader),
    Http(HttpReader),
    Https(HttpsReader),
    Stdin(StdinReader),
    Tcp(TcpReaderWriter),
    TcpListen(TcpListenReader),
    Tls(TlsReaderWriter),
    Udp(UdpReader),
}

/// Use the input implementation's respective Reader trait
impl InputReader for Reader {
    async fn read(&mut self) -> Result<Bytes, Error> {
        match self {
            Self::File(file_reader) => file_reader.read().await,
            Self::Http(http_reader) => http_reader.read().await,
            Self::Https(https_reader) => https_reader.read().await,
            Self::Stdin(stdin_reader) => stdin_reader.read().await,
            Self::Tcp(tcp_reader) => tcp_reader.read().await,
            Self::TcpListen(tcp_listen_reader) => tcp_listen_reader.read().await,
            Self::Tls(tls_reader) => tls_reader.read().await,
            Self::Udp(udp_reader) => udp_reader.read().await,
        }
    }
}

impl std::default::Default for Reader {
    fn default() -> Self {
        Self::Stdin(StdinReader::new())
    }
}
