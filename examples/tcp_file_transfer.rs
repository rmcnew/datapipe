/// # TCP File Transfer Example
///
/// Demonstrates using the datapipe library API to transfer a file over a TCP
/// connection on localhost. This example starts a TCP listener, then connects
/// to it and streams a file through the pipeline.
///
/// ## Usage
///
/// ```bash
/// cargo run --example tcp_file_transfer -- <input_file> <output_file>
/// ```
use datapipe::engine::run_datapipe;
use datapipe::file_reader::FileReader;
use datapipe::file_writer::FileWriter;
use datapipe::parameters::ParametersBuilder;
use datapipe::reader::Reader;
use datapipe::tcp_listen_reader::TcpListenReader;
use datapipe::tcp_reader_writer::TcpReaderWriter;
use datapipe::writer::Writer;
use std::path::PathBuf;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        return ExitCode::FAILURE;
    }

    // create PathBufs to give to child threads
    let input_path_string = args[1].clone();
    let output_path_string = args[2].clone();
    let input_path_buf = PathBuf::from(input_path_string.clone());
    let output_path_buf = PathBuf::from(output_path_string.clone());

    // Bind a TCP listener on an ephemeral port
    let tcp_listener = match TcpListenReader::new("localhost:0").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error binding TCP listener: {e}");
            return ExitCode::FAILURE;
        }
    };

    let listen_addr = match tcp_listener.get_listen_port() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Error getting listen address: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("TCP listener bound on {listen_addr}");

    // Spawn the receiving side: TCP listen -> file output
    let receiver_handle = tokio::spawn(async move {
        let writer = match FileWriter::new(&output_path_buf).await {
            Ok(w) => Writer::from(w),
            Err(e) => {
                eprintln!("Error opening output file: {e}");
                return;
            }
        };

        let parameters = match ParametersBuilder::new()
            .reader(Reader::from(tcp_listener))
            .writer(writer)
            .build()
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error building receiver parameters: {e}");
                return;
            }
        };

        match run_datapipe(parameters).await {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Error running TCP Listener: {err}");
            }
        }
    });

    // Brief delay to ensure the listener is ready
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Spawn the sending side: file input -> TCP connect
    let sender_handle = tokio::spawn(async move {
        let reader = match FileReader::new(&input_path_buf).await {
            Ok(r) => Reader::from(r),
            Err(e) => {
                eprintln!("Error opening input file: {e}");
                return;
            }
        };

        let tcp_writer = match TcpReaderWriter::new(&listen_addr.to_string()).await {
            Ok(w) => Writer::from(w),
            Err(e) => {
                eprintln!("Error connecting to TCP listener: {e}");
                return;
            }
        };

        let parameters = match ParametersBuilder::new()
            .reader(reader)
            .writer(tcp_writer)
            .build()
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error building sender parameters: {e}");
                return;
            }
        };

        match run_datapipe(parameters).await {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Error running TCP file sender: {err}");
            }
        }
    });

    // Wait for both sides to complete
    if let Err(e) = sender_handle.await {
        eprintln!("Sender task error: {e}");
    }
    if let Err(e) = receiver_handle.await {
        eprintln!("Receiver task error: {e}");
    }

    println!("TCP file transfer complete: {input_path_string} -> {output_path_string}");
    ExitCode::SUCCESS
}
