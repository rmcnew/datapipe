/// The main data read-write loop
use crate::datapipe_types::{DatapipeError, InputReader, OutputWriter, error_root_cause};
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::metrics::ByteMetrics;
use crate::parameters::Parameters;
use crate::reader::Reader;
use crate::writer::Writer;
use log::{error, info, warn};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::task::JoinSet;

const QUEUE_SIZE: usize = 2048;
const RETRY_MAX: i32 = 5; // retry failed reads or writes up to this many consecutive times before stopping

/// reader_child reads the input stream and pushes the read bytes into a queue.
/// When no bytes can be read, it briefly sleeps and retries until RETRY_MAX times
/// before quitting.  If an error occurs, it stops and returns the error.
async fn reader_child(
    mut reader: Reader,
    sender: Sender<Vec<u8>>,
    metrics: Option<ByteMetrics>,
) -> Result<(), DatapipeError> {
    let mut read_retry_count = 0;
    loop {
        match reader.read().await {
            // read success
            Ok(buffer) => {
                if buffer.is_empty() {
                    // retry a few times to make sure all of the input is read
                    read_retry_count += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    if read_retry_count >= RETRY_MAX {
                        let warning = "reader_child: no bytes read; stopping".to_string();
                        warn!("{warning}");
                        break;
                    }
                } else {
                    if let Some(ref m) = metrics {
                        m.add_bytes(buffer.len() as u64);
                    }
                    // buffer not empty; send the data
                    let v = buffer.to_vec();
                    match sender.send(v).await {
                        Ok(()) => {
                            read_retry_count = 0;
                        }
                        Err(err) => {
                            let error_message = format!(
                                "reader_child: cannot send to next stage: {}; stopping",
                                error_root_cause(&err)
                            );
                            error!("{error_message}");
                            return Err(DatapipeError::InputOutputError(error_message));
                        }
                    }
                }
            }
            // read error
            Err(err) => {
                // try again later
                read_retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                warn!(
                    "reader_child:  Error reading from input source: {}; read_retry_count is {read_retry_count}",
                    error_root_cause(&err)
                );
                // unless RETRY_MAX is reached; in that case, return error
                if read_retry_count >= RETRY_MAX {
                    let error_message = format!(
                        "reader_child:  RETRY_MAX {RETRY_MAX} reached; quitting due to repeated read errors"
                    );
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }
        }
    }
    Ok(())
}

/// decryptor_child receives an encrypted byte stream, queues enough data to decrypt,
/// performs decryption, verifies end-of-stream authentication on EOF, and forwards plaintext.
async fn decryptor_child(
    mut receiver: Receiver<Vec<u8>>,
    mut decryptor: StreamDecryptor,
    sender: Sender<Vec<u8>>,
) -> Result<(), DatapipeError> {
    let mut buffer: Vec<u8> = Vec::new();
    while let Some(bytes) = receiver.recv().await {
        buffer.extend_from_slice(&bytes);
        match decryptor.decrypt(&mut buffer) {
            Ok(plain) => {
                if !plain.is_empty()
                    && let Err(err) = sender.send(plain).await
                {
                    let error_message = format!(
                        "decryptor_child: cannot send to next stage: {}; stopping",
                        error_root_cause(&err)
                    );
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }
            Err(err) => {
                let error_message = format!(
                    "decryptor_child: error decrypting data: {}",
                    error_root_cause(&err)
                );
                error!("{error_message}");
                return Err(DatapipeError::EncryptionError(error_message));
            }
        }
    }

    decryptor.finish(&buffer)?;
    warn!("decryptor_child: end of encrypted stream reached; stopping");
    Ok(())
}

/// encryptor_child receives a data stream, encrypts each chunk into authenticated v2 frames,
/// emits the authenticated end-of-stream frame at EOF, and forwards ciphertext.
async fn encryptor_child(
    mut receiver: Receiver<Vec<u8>>,
    mut encryptor: StreamEncryptor,
    sender: Sender<Vec<u8>>,
) -> Result<(), DatapipeError> {
    let mut buffer: Vec<u8> = Vec::new();
    while let Some(bytes) = receiver.recv().await {
        buffer.extend_from_slice(&bytes);
        match encryptor.encrypt(&mut buffer) {
            Ok(cipher) => {
                if !cipher.is_empty()
                    && let Err(err) = sender.send(cipher).await
                {
                    let error_message = format!(
                        "encryptor_child: cannot send to next stage: {}; stopping",
                        error_root_cause(&err)
                    );
                    error!("{error_message}");
                    return Err(DatapipeError::InputOutputError(error_message));
                }
            }
            Err(error) => {
                let error_message = format!(
                    "encryptor_child: error encrypting data: {}",
                    error_root_cause(&error)
                );
                error!("{error_message}");
                return Err(DatapipeError::EncryptionError(error_message));
            }
        }
    }

    let final_frame = encryptor.finish()?;
    if !final_frame.is_empty()
        && let Err(err) = sender.send(final_frame).await
    {
        let error_message = format!(
            "encryptor_child: cannot send final encrypted EOF frame to next stage: {}",
            error_root_cause(&err)
        );
        error!("{error_message}");
        return Err(DatapipeError::InputOutputError(error_message));
    }
    warn!("encryptor_child: finalized encrypted stream; stopping");
    Ok(())
}

async fn close_all_writers(writers: &mut [Writer]) {
    for (idx, writer) in writers.iter_mut().enumerate() {
        if let Err(err) = writer.close().await {
            let error_cause = error_root_cause(&err);
            warn!("writer_child: Error closing output writer #{idx}: {error_cause}");
        }
    }
}

/// writer_child receives a byte stream and writes each chunk to all configured writers.
/// Consecutive errors are tracked per writer; if any writer reaches `RETRY_MAX` consecutive
/// failures, all writers are closed cleanly and an error is returned.
async fn writer_child(
    mut receiver: Receiver<Vec<u8>>,
    mut writers: Vec<Writer>,
    metrics: Option<ByteMetrics>,
) -> Result<(), DatapipeError> {
    let mut write_retry_counts = vec![0i32; writers.len()];
    while let Some(bytes) = receiver.recv().await {
        if bytes.is_empty() {
            continue;
        }
        for (idx, writer) in writers.iter_mut().enumerate() {
            match writer.write(&bytes).await {
                Ok(()) => {
                    if let Some(ref m) = metrics {
                        m.add_bytes(bytes.len() as u64);
                    }
                    write_retry_counts[idx] = 0;
                }
                Err(err) => {
                    let error_cause = error_root_cause(&err);
                    write_retry_counts[idx] += 1;
                    error!(
                        "writer_child: Error writing to output #{idx}: {error_cause}; consecutive failure count is {}",
                        write_retry_counts[idx]
                    );
                    if write_retry_counts[idx] >= RETRY_MAX {
                        let error_message = format!(
                            "writer_child: output #{idx} reached RETRY_MAX {RETRY_MAX}; quitting due to repeated write errors: {error_cause}"
                        );
                        error!("{error_message}");
                        close_all_writers(&mut writers).await;
                        return Err(DatapipeError::InputOutputError(error_message));
                    }
                }
            }
        }
    }
    warn!("writer_child: no more input received; stopping");
    close_all_writers(&mut writers).await;
    Ok(())
}

/// library API entry point: just supply parameters and run it
pub async fn run_datapipe(parameters: Parameters) -> Result<(), DatapipeError> {
    let mut join_set: JoinSet<Result<(), DatapipeError>> = JoinSet::new();
    // setup queue from reader thread to writer thread
    let (reader_sender, reader_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);

    let Parameters {
        reader,
        maybe_decryptor,
        maybe_encryptor,
        writers,
        metrics,
    } = parameters;

    let (reader_metrics, writer_metrics) = match metrics {
        Some(m) => (Some(m.reader_bytes), Some(m.writer_bytes)),
        None => (None, None),
    };

    // spawn stages:
    // 1) reader stage to read input bytes and push into queue
    join_set.spawn(reader_child(reader, reader_sender, reader_metrics));

    // 2) optional decryption / encryption stages and 3) writer stage
    match (maybe_decryptor, maybe_encryptor) {
        (Some(decryptor), Some(encryptor)) => {
            let (decryptor_sender, decryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);
            let (encryptor_sender, encryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);
            join_set.spawn(decryptor_child(
                reader_receiver,
                decryptor,
                decryptor_sender,
            ));
            join_set.spawn(encryptor_child(
                decryptor_receiver,
                encryptor,
                encryptor_sender,
            ));
            join_set.spawn(writer_child(encryptor_receiver, writers, writer_metrics));
        }
        (Some(decryptor), None) => {
            let (decryptor_sender, decryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);
            join_set.spawn(decryptor_child(
                reader_receiver,
                decryptor,
                decryptor_sender,
            ));
            join_set.spawn(writer_child(decryptor_receiver, writers, writer_metrics));
        }
        (None, Some(encryptor)) => {
            let (encryptor_sender, encryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);
            join_set.spawn(encryptor_child(
                reader_receiver,
                encryptor,
                encryptor_sender,
            ));
            join_set.spawn(writer_child(encryptor_receiver, writers, writer_metrics));
        }
        (None, None) => {
            join_set.spawn(writer_child(reader_receiver, writers, writer_metrics));
        }
    }

    info!("main thread: waiting for child tasks to finish");
    while let Some(join_result) = join_set.join_next().await {
        match join_result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                error!("{err}");
                join_set.abort_all();
                return Err(err);
            }
            Err(join_err) => {
                error!("{join_err}");
                join_set.abort_all();
                return Err(join_err.into());
            }
        }
    }
    Ok(())
}
