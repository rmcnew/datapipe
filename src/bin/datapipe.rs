// Do simple data forwarding from one input to one or more outputs
use clap::Parser;
use datapipe::args::ProgramArgs;
use datapipe::engine::run_data_pipe;
use datapipe::logger::init_logger;
use log::info;
use std::process::ExitCode;


#[tokio::main]
async fn main() -> ExitCode {
    // Setup default crypto provider
    rustls::crypto::ring::default_provider().install_default().expect("Failed to install ring as rustls crypto provider");

    let args = ProgramArgs::parse();
    let log_dir = match args.logging_args.log_dir {
        Some(ref log_dir_string) => log_dir_string,
        None => "/var/tmp"
    };
    let _log_handle = match init_logger(&log_dir, "datapipe", args.logging_args.keep_logs) {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("Logger error: {}", error);
            return ExitCode::FAILURE;
        }
    };
    info!("Args are: {:?}", args);
    match args.to_parameters().await {
        Ok(parameters) => {
            run_data_pipe(parameters).await;
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}", error);
            ExitCode::FAILURE
        }
    }

}
