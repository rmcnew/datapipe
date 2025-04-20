/// Writer for STDOUT
use crate::datapipe_types::OutputWriter;
use std::io::Error;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct StdoutWriter {}

impl StdoutWriter {
    pub fn new() -> Self {
        Self {}
    }
}

impl OutputWriter for StdoutWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        tokio::io::stdout().write_all(bytes).await
    }
}
