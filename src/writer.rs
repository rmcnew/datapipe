use crate::datapipe_types::OutputWriter;
use crate::file_writer::FileWriter;
use crate::http_writer::HttpWriter;
use crate::https_writer::HttpsWriter;
use crate::stdout_writer::StdoutWriter;
use crate::tcp_reader_writer::TcpReaderWriter;
use crate::tls_reader_writer::TlsReaderWriter;
use crate::udp_writer::UdpWriter;
use std::io::Error;

/// Writer enum wraps all output implementations
#[derive(Debug)]
pub enum Writer {
    File(FileWriter),
    Http(HttpWriter),
    Https(HttpsWriter),
    Stdout(StdoutWriter),
    Tcp(TcpReaderWriter),
    Tls(Box<TlsReaderWriter>),
    Udp(UdpWriter),
}

/// Use the output implmentation's respective OutputWriter trait
impl OutputWriter for Writer {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match self {
            Self::File(file_writer) => file_writer.write(bytes).await,
            Self::Http(http_writer) => http_writer.write(bytes).await,
            Self::Https(https_writer) => https_writer.write(bytes).await,
            Self::Stdout(stdout_writer) => stdout_writer.write(bytes).await,
            Self::Tcp(tcp_writer) => tcp_writer.write(bytes).await,
            Self::Tls(tls_writer) => tls_writer.write(bytes).await,
            Self::Udp(udp_writer) => udp_writer.write(bytes).await,
        }
    }
}

/// The default Writer is STDOUT
impl std::default::Default for Writer {
    fn default() -> Self {
        Self::Stdout(StdoutWriter::new())
    }
}

/// Convert a FileWriter to a Writer
impl std::convert::From<FileWriter> for Writer {
    fn from(value: FileWriter) -> Self {
        Self::File(value)
    }
}

/// Convert an HttpWriter to a Writer
impl std::convert::From<HttpWriter> for Writer {
    fn from(value: HttpWriter) -> Self {
        Self::Http(value)
    }
}

/// Convert an HttpsWriter to a Writer
impl std::convert::From<HttpsWriter> for Writer {
    fn from(value: HttpsWriter) -> Self {
        Self::Https(value)
    }
}

/// Convert a StdoutWriter to a Writer
impl std::convert::From<StdoutWriter> for Writer {
    fn from(value: StdoutWriter) -> Self {
        Self::Stdout(value)
    }
}

/// Convert a TcpReaderWriter to a Writer
impl std::convert::From<TcpReaderWriter> for Writer {
    fn from(value: TcpReaderWriter) -> Self {
        Self::Tcp(value)
    }
}

/// Convert a TlsReaderWriter to a Writer
impl std::convert::From<TlsReaderWriter> for Writer {
    fn from(value: TlsReaderWriter) -> Self {
        Self::Tls(Box::new(value))
    }
}

/// Convert a UdpWriter to a Writer
impl std::convert::From<UdpWriter> for Writer {
    fn from(value: UdpWriter) -> Self {
        Self::Udp(value)
    }
}
