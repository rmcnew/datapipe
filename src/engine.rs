/// The main data read-write loop
use crate::datapipe_types::{InputReader, OutputWriter};
use crate::reader::Reader;
use crate::writer::Writer;
use log::{error, info, trace, warn};
use tokio::sync::mpsc::channel;

const QUEUE_SIZE: usize = 2048;
const RETRY_MAX: i32 = 5; // retry failed reads or writes up to this many consecutive times before stopping

/// This struct gives all the parameters needed to start a datapipe instance
pub struct Parameters {
    pub reader: Reader,
    pub writers: Vec<Writer>
}

impl std::default::Default for Parameters {
    fn default() -> Self {
        Self {
            reader: Reader::default(),
            writers: vec![Writer::default()]
        }
    }
}

pub async fn run_data_pipe(parameters: Parameters) {
    
    // vec to track child threads
    let mut children = Vec::new();    
    // setup queue from reader thread to writer thread
    let (queue_sender, mut queue_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);

    // spawn threads:
    // 1)  reader thread to get byte input and place in input queue
    let reader_handle = tokio::spawn(async move {
        let mut reader = parameters.reader;
        let mut read_retry_count = 0;            
        loop {
            match reader.read().await {
                Ok(buffer) => {
                    if buffer.is_empty() {
                        warn!("reader thread: No bytes read");
                        break;
                    } else {
                        trace!("reader thread: read bytes {:?}", &buffer);
                    }
                    let v = buffer.to_vec();
                    match queue_sender.send(v).await {
                        Ok(()) => {}
                        Err(_error) => {
                            warn!("reader thread: writer thread shut down.  Stopping.");
                            break;
                        }
                    }
                    // reset retry count
                    read_retry_count = 0;
                }
                Err(error) => {
                    read_retry_count += 1;
                    warn!("reader thread:  Error reading from input source: {}; read_retry_count is {}", error, read_retry_count);
                    if read_retry_count >= RETRY_MAX {
                        let error_message = format!("reader thread:  RETRY_MAX {} reached; quitting due to repeated read errors", RETRY_MAX);
                        error!("{}", error_message);
                        eprintln!("{}", error_message);
                        break;
                    }
                }
            }
        }
        warn!("reader thread: stopping");
        
    });
    children.push(reader_handle);
    // 2)  writer thread to write output queue
    let writer_handle = tokio::spawn(async move {
        let mut writers = parameters.writers;
        let mut write_retry_count = 0;            
        'writer: loop {
            match queue_receiver.recv().await {
                Some(bytes) => {
                    for writer in &mut writers {
                        match writer.write(&bytes).await {
                            Ok(()) => {
                                trace!("writer thread: wrote bytes");
                                write_retry_count = 0;
                            }
                            Err(error) => {
                                write_retry_count += 1;
                                warn!("writer thread:  Error writing string to output: {}; write_retry_count is {}", error, write_retry_count);
                                if write_retry_count >= RETRY_MAX {
                                    let error_message = format!("writer thread: RETRY_MAX {} reached; quitting due to repeated write errors", RETRY_MAX);
                                    error!("{}", error_message);
                                    eprintln!("{}", error_message);
                                    break 'writer;
                                }
                            }
                        }
                    }
                }
                None => {
                    warn!("writer thread: converter thread shut down.  Stopping.");
                    break;
                }
            }
        }
        warn!("writer thread: stopping");          
    });
    children.push(writer_handle);

    info!("main thread: waiting for child threads to finish");
    for child in children {
        match child.await {
            Ok(()) => {}
            Err(error) => {
                eprintln!("{}", error);
            }
        }
    }
}