use crate::datapipe_types::OutputWriter;
use crate::file_writer::FileWriter;
use crate::http_writer::HttpWriter;
use crate::https_writer::HttpsWriter;
use crate::stdout_writer::StdoutWriter;
use crate::tcp_reader_writer::TcpReaderWriter;
use crate::tls_reader_writer::TlsReaderWriter;
use crate::udp_writer::UdpWriter;
use std::io::Error;

// Combined writer
#[derive(Debug)]
pub enum Writer {
    File(FileWriter),
    Http(HttpWriter),
    Https(HttpsWriter),
    Stdout(StdoutWriter),
    Tcp(TcpReaderWriter),
    Tls(TlsReaderWriter),
    Udp(UdpWriter),
}

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

impl std::default::Default for Writer {
    fn default() -> Self {
        Self::Stdout(StdoutWriter::new())
    }
}
