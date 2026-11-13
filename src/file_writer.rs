/// Writer for file output
use crate::datapipe_types::OutputWriter;
use std::io::Error;
use std::path::Path;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct FileWriter {
    file: tokio::fs::File,
}

impl FileWriter {
    pub async fn new(pathref: &Path) -> Result<Self, Error> {
        Ok(Self {
            file: tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(pathref)
                .await?,
        })
    }
}

impl OutputWriter for FileWriter {
    async fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.file.write_all(bytes).await
    }

    async fn close(&mut self) -> Result<(), Error> {
        self.file.flush().await?;
        self.file.shutdown().await
    }
}
