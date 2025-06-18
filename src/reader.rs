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

/// Reader enum wraps all input implementations
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

/// Use the input implementation's respective InputReader trait
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

/// The default Reader is for STDIN
impl std::default::Default for Reader {
    fn default() -> Self {
        Self::Stdin(StdinReader::new())
    }
}

/// Convert a FileReader to a Reader
impl std::convert::From<FileReader> for Reader {
    fn from(value: FileReader) -> Self {
        Self::File(value)
    }
}

/// Convert an HttpReader to a Reader
impl std::convert::From<HttpReader> for Reader {
    fn from(value: HttpReader) -> Self {
        Self::Http(value)
    }
}

/// Convert an HttpsReader to a Reader
impl std::convert::From<HttpsReader> for Reader {
    fn from(value: HttpsReader) -> Self {
        Self::Https(value)
    }
}

/// Convert a StdinReader to a Reader
impl std::convert::From<StdinReader> for Reader {
    fn from(value: StdinReader) -> Self {
        Self::Stdin(value)
    }
}

/// Convert a TcpReaderWriter to a Reader
impl std::convert::From<TcpReaderWriter> for Reader {
    fn from(value: TcpReaderWriter) -> Self {
        Self::Tcp(value)
    }
}

/// Convert a TcpListenReader to a Reader
impl std::convert::From<TcpListenReader> for Reader {
    fn from(value: TcpListenReader) -> Self {
        Self::TcpListen(value)
    }
}

/// Convert a TlsReaderWriter to a Reader
impl std::convert::From<TlsReaderWriter> for Reader {
    fn from(value: TlsReaderWriter) -> Self {
        Self::Tls(value)
    }
}

/// Convert a UdpReader to a Reader
impl std::convert::From<UdpReader> for Reader {
    fn from(value: UdpReader) -> Self {
        Self::Udp(value)
    }
}
