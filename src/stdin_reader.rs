use crate::datapipe_types::InputReader;
/// Reader for STDIN
use bytes::{Bytes, BytesMut};
use std::io::Error;
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub struct StdinReader {}

impl StdinReader {
    pub fn new() -> Self {
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
