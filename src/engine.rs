/// The main data read-write loop
use crate::datapipe_types::{error_root_cause, InputReader, OutputWriter};
use crate::encryption::{StreamDecryptor, StreamEncryptor};
use crate::parameters::Parameters;
use crate::reader::Reader;
use crate::writer::Writer;
use log::{error, info, warn};
use tokio::sync::mpsc::{Receiver, Sender, channel};

const QUEUE_SIZE: usize = 2048;
const RETRY_MAX: i32 = 5; // retry failed reads or writes up to this many consecutive times before stopping

async fn reader_child(mut reader: Reader, sender: Sender<Vec<u8>>) {
    let mut read_retry_count = 0;
    loop {
        match reader.read().await {
            Ok(buffer) => {
                if buffer.is_empty() {
                    // retry a few times to make sure all of the input is read
                    read_retry_count += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    if read_retry_count >= RETRY_MAX {
                        warn!("reader_child: no bytes read; stopping");
                        break;
                    }
                } else { // buffer not empty send the data
                    let v = buffer.to_vec();
                    match sender.send(v).await {
                        Ok(()) => {
                            read_retry_count = 0;
                        }
                        Err(_error) => {
                            warn!("reader_child: cannot send to next stage; stopping");
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                read_retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                warn!(
                    "reader_child:  Error reading from input source: {error}; read_retry_count is {read_retry_count}"
                );
                if read_retry_count >= RETRY_MAX {
                    let error_message = format!(
                        "reader_child:  RETRY_MAX {RETRY_MAX} reached; quitting due to repeated read errors"
                    );
                    error!("{error_message}");
                    break;
                }
            }
        }
    }
}

async fn decryptor_child(
    mut receiver: Receiver<Vec<u8>>,
    mut decryptor: StreamDecryptor,
    sender: Sender<Vec<u8>>,
) {
    let mut buffer: Vec<u8> = Vec::new();
    let mut retry_count = 0;
    loop {
        match receiver.recv().await {
            Some(bytes) => {
                retry_count = 0;
                buffer.extend_from_slice(&bytes);
                match decryptor.decrypt(&mut buffer) {
                    Ok(plain) => match sender.send(plain).await {
                        Ok(()) => {}
                        Err(_error) => {
                            warn!("decryptor_child: cannot send to next stage; stopping");
                            break;
                        }
                    },
                    Err(error) => {
                        let error_message = format!("decryptor_child: error decrypting data: {}", error_root_cause(&error));
                        error!("{error_message}");
                        eprintln!("{error_message}");
                        break;
                    }
                }
            },
            None => {
                retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                if retry_count >= RETRY_MAX {
                    warn!("decryptor_child: no bytes received; stopping");
                    break;
                }
            }
        }
    }
}

async fn encryptor_child(
    mut receiver: Receiver<Vec<u8>>,
    mut encryptor: StreamEncryptor,
    sender: Sender<Vec<u8>>,
) {
    let mut buffer: Vec<u8> = Vec::new();
    let mut retry_count = 0;
    loop {
        match receiver.recv().await {
            Some(bytes) => {
                retry_count = 0;
                buffer.extend_from_slice(&bytes);
                match encryptor.encrypt(&mut buffer) {
                    Ok(cipher) => match sender.send(cipher).await {
                        Ok(()) => {}
                        Err(_error) => {
                            warn!("encryptor_child: cannot send to next stage; stopping");
                            break;
                        }
                    },
                    Err(error) => {
                        let error_message = format!("encryptor_child: error encrypting data: {}", error_root_cause(&error));
                        error!("{error_message}");
                        eprintln!("{error_message}");
                        break;
                    }
                }
            },
            None => {
                retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                if retry_count >= RETRY_MAX {
                    warn!("encryptor_child: no bytes received; stopping");
                    break;
                }
            }
        }
    }
}

async fn writer_child(mut receiver: Receiver<Vec<u8>>, mut writers: Vec<Writer>) {
    let mut write_retry_count = 0;
    'writer: loop {
        match receiver.recv().await {
            Some(bytes) => {
                if !bytes.is_empty() {
                    for writer in &mut writers {
                        match writer.write(&bytes).await {
                            Ok(()) => {
                                // if at least one writer is working, continue
                                write_retry_count = 0;
                            }
                            Err(error) => {
                                // should the count be per output sink?
                                let error_cause = error_root_cause(&error);
                                write_retry_count += 1;
                                warn!("writer_child:  Error writing to output: {error_cause}; write_retry_count is {write_retry_count}");
                                if write_retry_count >= RETRY_MAX {
                                    let error_message = format!(
                                        "writer_child: RETRY_MAX {RETRY_MAX} reached; quitting due to repeated write errors"
                                    );
                                    error!("{error_message}");
                                    eprintln!("{error_message}");
                                    break 'writer;
                                }
                            }
                        }
                    }
                }
            }
            None => {
                // retry a few times before quitting to ensure all the output gets written
                write_retry_count += 1;
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                if write_retry_count >= RETRY_MAX {
                    warn!("writer_child: stopping");
                    break;
                }
            }
        }
    }
}

/// library API entry point: just supply parameters and run it
pub async fn run_data_pipe(parameters: Parameters) {
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
            Ok(()) => {}
            Err(error) => {
                eprintln!("{error}");
            }
        }
    }
}
