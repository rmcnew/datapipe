// Do simple data forwarding from one input to one or more outputs
use clap::Parser;
use datapipe::args::ProgramArgs;
use datapipe::logger::init_logger;
use datapipe::reader::{InputReader, get_input_reader};
use datapipe::writer::{OutputWriter, get_output_writers};
use log::{error, info, trace, warn};
use std::net::ToSocketAddrs;
use std::process::ExitCode;
use tokio::sync::mpsc::channel;
use url::Url;


fn good_url(maybe_url: &str) -> bool {
    if maybe_url.starts_with("http://") || maybe_url.starts_with("https://") {
        match Url::parse(maybe_url) {
            Ok(_url) => return true,
            Err(_parse_error) => return false,
        }
    }
    eprintln!("Output URL must start with 'http://' or 'https://'");
    false
}

fn good_tcp_udp_address(maybe_udp_address: &str) -> bool {
    match maybe_udp_address.to_socket_addrs() {
        Ok(_socket_addrs) => true,
        Err(_error) => false,
    }
}


const QUEUE_SIZE: usize = 2048;
const RETRY_MAX: i32 = 5; // retry failed reads or writes up to this many consecutive times before stopping
#[tokio::main]
async fn main() -> ExitCode {
    let args = ProgramArgs::parse();
    let _log_handle = match init_logger("/var/tmp", "datapipe") {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("Logger error: {}", error);
            return ExitCode::FAILURE;
        }
    };
    info!("Args are: {:?}", args);
    // parse input UDP address if present
    if args.input.udp_input.is_some()
        && !good_tcp_udp_address(args.input.udp_input.clone().unwrap().as_str())
    {
        eprintln!(
            "Invalid UDP input address: {}",
            args.input.udp_input.unwrap()
        );
        return ExitCode::FAILURE;
    }
    // parse HTTP input URL if present
    if args.input.http_input.is_some() && !good_url(args.input.http_input.clone().unwrap().as_str())
    {
        eprintln!("Invalid HTTP input URL: {}", args.input.http_input.unwrap());
        return ExitCode::FAILURE;
    }
    // parse output UDP address if present
    if args.output.udp_output.is_some() {
        let addresses = args.output.udp_output.clone().unwrap();
        for address in addresses {
            if !good_tcp_udp_address(&address) {
                eprintln!("Invalid UDP output address: {}", address);
                return ExitCode::FAILURE;
            }
        }
    }
    // parse HTTP output URL if present
    if args.output.http_output.is_some()
        && !good_url(args.output.http_output.clone().unwrap().as_str())
    {
        eprintln!(
            "Invalid HTTP output URL: {}",
            args.output.http_output.unwrap()
        );
        return ExitCode::FAILURE;
    }
    // parse TLS (TCP) address if present
    if args.output.tls_output.is_some() {
        let addresses = args.output.tls_output.clone().unwrap();
        for address in addresses {
            if !good_tcp_udp_address(&address) {
                eprintln!("Invalid TLS output address: {}", address);
                return ExitCode::FAILURE;
            }
        }
    }

    // vec to track child threads
    let mut children = Vec::new();
    // clone args for use by reader
    let args_clone = args.clone();
    // setup queue from reader thread to writer thread
    let (queue_sender, mut queue_receiver) = channel::<Vec<u8>>(QUEUE_SIZE);

    // spawn threads:
    // 1)  reader thread to get byte input and place in input queue
    let reader = tokio::spawn(async move {
        let mut read_retry_count = 0;
        match get_input_reader(&args_clone).await {
            Ok(mut reader) => {
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
            }
            Err(error) => {
                eprintln!("reader thread:  Error initializing input reader: {}", error);
            }
        }
    });
    children.push(reader);
    // 2)  writer thread to write output queue
    let writer_handle = tokio::spawn(async move {
        let mut write_retry_count = 0;
        match get_output_writers(&args).await {
            Ok(mut writers) => {
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
            }
            Err(error) => {
                eprintln!(
                    "writer thread:  Error initializing output writer: {}",
                    error
                );
            }
        }
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

    ExitCode::SUCCESS
}
