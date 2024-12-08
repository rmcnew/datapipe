// Do simple data forwarding from one input to one or more outputs
use clap::Parser;
use datapipe::args::ProgramArgs;
use datapipe::engine::run_data_pipe;
use datapipe::logger::init_logger;
use log::info;
use std::process::ExitCode;


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
