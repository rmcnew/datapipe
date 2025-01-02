/// Reader for files

use bytes::{Bytes, BytesMut};
use crate::datapipe_types::InputReader;
use log::trace;
use std::path::Path;
use std::io::{Error, SeekFrom};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

#[derive(Debug)]
pub struct FileReader {
    file: tokio::fs::File,
}

impl FileReader {
    pub async fn new(pathref: &Path) -> Result<Self, Error> {
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
