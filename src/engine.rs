/// The main data read-write loop
use crate::datapipe_types::{DatapipeError, InputReader, OutputWriter, error_root_cause};
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::parameters::Parameters;
use crate::reader::Reader;
use crate::writer::Writer;
use log::{error, info, warn};
use tokio::sync::mpsc::{Receiver, Sender, channel};

const QUEUE_SIZE: usize = 2048;
const RETRY_MAX: i32 = 5; // retry failed reads or writes up to this many consecutive times before stopping

/// reader_child reads the input stream and pushes the read bytes into a queue.
/// When no bytes can be read, it briefly sleeps and retries until RETRY_MAX times
/// before quitting.  If an error occurs, it stops and returns the error.
async fn reader_child(mut reader: Reader, sender: Sender<Vec<u8>>) -> Result<(), DatapipeError> {
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
/// performs decryption, and sends the decrypted byte stream
async fn decryptor_child(
    mut receiver: Receiver<Vec<u8>>,
    mut decryptor: StreamDecryptor,
    sender: Sender<Vec<u8>>,
) -> Result<(), DatapipeError> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut retry_count = 0;
    loop {
        match receiver.recv().await {
            // receive the encrypted byte stream
            Some(bytes) => {
                retry_count = 0;
                buffer.extend_from_slice(&bytes);
                match decryptor.decrypt(&mut buffer) {
                    // decrypt success, send the decrypted data
                    Ok(plain) => match sender.send(plain).await {
                        // data send success
                        Ok(()) => {}
                        // data send error; return error
                        Err(err) => {
                            let error_message = format!(
                                "decryptor_child: cannot send to next stage: {}; stopping",
                                error_root_cause(&err)
                            );
                            error!("{error_message}");
                            return Err(DatapipeError::InputOutputError(error_message));
                        }
                    },
                    // decrypt error, return error
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
            // no data received, wait and try again later
            None => {
                retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                // unless RETRY_MAX reached; then stop
                if retry_count >= RETRY_MAX {
                    let warning = "decryptor_child: no bytes received; stopping".to_string();
                    warn!("{warning}");
                    break;
                }
            }
        }
    }
    Ok(())
}

/// encryptor_child receives a data stream, queues enough data to encrypt,
/// encrypts the data, and then sends the encrypted data stream
async fn encryptor_child(
    mut receiver: Receiver<Vec<u8>>,
    mut encryptor: StreamEncryptor,
    sender: Sender<Vec<u8>>,
) -> Result<(), DatapipeError> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut retry_count = 0;
    loop {
        match receiver.recv().await {
            // receive success
            Some(bytes) => {
                retry_count = 0;
                buffer.extend_from_slice(&bytes);
                match encryptor.encrypt(&mut buffer) {
                    // encrypt success, send the encrypted data
                    Ok(cipher) => match sender.send(cipher).await {
                        // send success
                        Ok(()) => {}
                        // send error, return error
                        Err(err) => {
                            let error_message = format!(
                                "encryptor_child: cannot send to next stage: {}; stopping",
                                error_root_cause(&err)
                            );
                            error!("{error_message}");
                            return Err(DatapipeError::InputOutputError(error_message));
                        }
                    },
                    // encrypt error, return error
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
            // no bytes received, wait and try again later
            None => {
                retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                // unless RETRY_MAX reached, then stop
                if retry_count >= RETRY_MAX {
                    let warning = "encryptor_child: no bytes received; stopping".to_string();
                    warn!("{warning}");
                    break;
                }
            }
        }
    }
    Ok(())
}

/// writer_child receives a byte stream and writes the byte stream to
/// all writers; as long as one writer in the last RETRY_MAX writers
/// is successful, it will continue receiving and writing
async fn writer_child(
    mut receiver: Receiver<Vec<u8>>,
    mut writers: Vec<Writer>,
) -> Result<(), DatapipeError> {
    let mut write_retry_count = 0;
    loop {
        match receiver.recv().await {
            // receive success
            Some(bytes) => {
                if !bytes.is_empty() {
                    // iterate through all the writers
                    for writer in &mut writers {
                        match writer.write(&bytes).await {
                            // write success
                            Ok(()) => {
                                // if at least one writer in the last
                                // RETRY_MAX writers is working, continue
                                write_retry_count = 0;
                            }
                            // write error, log the error and continue
                            Err(err) => {
                                let error_cause = error_root_cause(&err);
                                write_retry_count += 1;
                                error!(
                                    "writer_child:  Error writing to output: {error_cause}; write_retry_count is {write_retry_count}"
                                );
                                // unless RETRY_MAX reached, return error
                                if write_retry_count >= RETRY_MAX {
                                    let error_message = format!(
                                        "writer_child: RETRY_MAX {RETRY_MAX} reached; quitting due to repeated write errors"
                                    );
                                    error!("{error_message}");
                                    return Err(DatapipeError::InputOutputError(error_message));
                                }
                            }
                        }
                    }
                }
            }
            // no data received, wait and try again later
            None => {
                // retry a few times before quitting to ensure all the output gets written
                write_retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                // until RETRY_MAX is reached, the stop
                if write_retry_count >= RETRY_MAX {
                    let warning = "writer_child: no more input received; stopping".to_string();
                    warn!("{warning}");
                    break;
                }
            }
        }
    }
    Ok(())
}

/// library API entry point: just supply parameters and run it
pub async fn run_datapipe(parameters: Parameters) -> Result<(), DatapipeError> {
    // vec to track child threads
    let mut children = Vec::new();
    // setup queue from reader thread to writer thread
    let (reader_sender, reader_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);

    let Parameters {
        reader,
        maybe_decryptor,
        maybe_encryptor,
        writers,
    } = parameters;

    // spawn threads:
    // 1)  reader thread to get byte input and place in input queue
    let reader_handle = tokio::spawn(reader_child(reader, reader_sender));
    children.push(reader_handle);
    // this is messy, but we don't want to add empty stages or unnecessary queues
    // is there a better way to build a dynamic pipeline of stages?
    // 2)  Decryption thread (if specified)  and   3)  Encryption thread (if specified)
    // 4)  writer thread to write output queue (always)
    match maybe_decryptor {
        Some(decryptor) => {
            match maybe_encryptor {
                Some(encryptor) => {
                    // decryptor and encryptor
                    let (decryptor_sender, decryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);
                    let (encryptor_sender, encryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);

                    let decryptor_handle = tokio::spawn(decryptor_child(
                        reader_receiver,
                        decryptor,
                        decryptor_sender,
                    ));
                    children.push(decryptor_handle);

                    let encryptor_handle = tokio::spawn(encryptor_child(
                        decryptor_receiver,
                        encryptor,
                        encryptor_sender,
                    ));
                    children.push(encryptor_handle);

                    let writer_handle = tokio::spawn(writer_child(encryptor_receiver, writers));
                    children.push(writer_handle);
                }
                None => {
                    // decryptor only
                    let (decryptor_sender, decryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);

                    let decryptor_handle = tokio::spawn(decryptor_child(
                        reader_receiver,
                        decryptor,
                        decryptor_sender,
                    ));
                    children.push(decryptor_handle);

                    let writer_handle = tokio::spawn(writer_child(decryptor_receiver, writers));
                    children.push(writer_handle);
                }
            }
        }
        None => {
            match maybe_encryptor {
                Some(encryptor) => {
                    // encryptor only
                    let (encryptor_sender, encryptor_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);

                    let encryptor_handle = tokio::spawn(encryptor_child(
                        reader_receiver,
                        encryptor,
                        encryptor_sender,
                    ));
                    children.push(encryptor_handle);

                    let writer_handle = tokio::spawn(writer_child(encryptor_receiver, writers));
                    children.push(writer_handle);
                }
                None => {
                    // neither decryptor nor encryptor
                    let writer_handle = tokio::spawn(writer_child(reader_receiver, writers));
                    children.push(writer_handle);
                }
            }
        }
    }

    info!("main thread: waiting for child threads to finish");
    for child in children {
        match child.await {
            Ok(child_result) => match child_result {
                Ok(()) => {}
                Err(err) => {
                    error!("{err}");
                    return Err(err);
                }
            },
            Err(err) => {
                error!("{err}");
                return Err(err.into());
            }
        }
    }
    Ok(())
}
