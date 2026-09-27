// Do simple data forwarding from one input to one or more outputs
use clap::Parser;
use datapipe::args::ProgramArgs;
use datapipe::config::DatapipeConfig;
use datapipe::engine::run_datapipe;
use datapipe::logger::init_logger;
use log::info;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = ProgramArgs::parse();

    // If --version is passed, print version, name, and copyright, then exit
    if args.version {
        println!("{}", datapipe::args::version_info());
        return ExitCode::SUCCESS;
    }

    // If --license is passed, print embedded license and exit
    if args.license {
        print!("{}", datapipe::args::license_info());
        return ExitCode::SUCCESS;
    }

    // Setup default crypto provider
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        eprintln!("Failed to install ring as rustls crypto provider");
        return ExitCode::FAILURE;
    }

    // If --verify-config is passed, validate the configuration file and exit
    if let Some(ref config_path) = args.verify_config {
        if let Err(error) = args.validate() {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
        match DatapipeConfig::load_from_file(config_path) {
            Ok(config) => match config.validate() {
                Ok(()) => {
                    println!("Configuration file {:?} is valid.", config_path);
                    return ExitCode::SUCCESS;
                }
                Err(error) => {
                    eprintln!("Configuration validation error: {error}");
                    return ExitCode::FAILURE;
                }
            },
            Err(error) => {
                eprintln!(
                    "Error loading configuration file {:?}: {error}",
                    config_path
                );
                return ExitCode::FAILURE;
            }
        }
    }

    // If --save-to-config is passed, validate CLI args, save to file, and exit
    if let Some(ref save_path) = args.save_to_config {
        if let Err(error) = args.validate() {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
        let config = DatapipeConfig::from(&args);
        if let Err(error) = config.validate() {
            eprintln!("Configuration validation error: {error}");
            return ExitCode::FAILURE;
        }
        if let Err(error) = config.save_to_file(save_path) {
            eprintln!("Error saving configuration to {:?}: {error}", save_path);
            return ExitCode::FAILURE;
        }
        println!("Configuration saved to {:?}", save_path);
        return ExitCode::SUCCESS;
    }

    // If --use-config is passed, load and validate the configuration file
    if let Some(ref config_path) = args.use_config {
        if let Err(error) = args.validate() {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
        let config = match DatapipeConfig::load_from_file(config_path) {
            Ok(config) => config,
            Err(error) => {
                eprintln!(
                    "Error loading configuration file {:?}: {error}",
                    config_path
                );
                return ExitCode::FAILURE;
            }
        };
        if let Err(error) = config.validate() {
            eprintln!("Configuration validation error: {error}");
            return ExitCode::FAILURE;
        }
        let cli_no_metrics = args.no_metrics;
        args = ProgramArgs::from(config);
        if cli_no_metrics {
            args.no_metrics = true;
        }
    } else {
        // Standard CLI execution without config file
        if let Err(error) = args.validate() {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    }

    let log_dir = match args.logging_args.log_dir {
        Some(ref log_dir_string) => log_dir_string,
        None => "/var/tmp",
    };
    let _log_handle = match init_logger(log_dir, "datapipe", args.logging_args.keep_logs) {
        Ok(handle) => handle,
        Err(err) => {
            eprintln!("Logger error: {}", err);
            return ExitCode::FAILURE;
        }
    };
    info!("Args are: {:?}", args);

    let parameters = match args.to_parameters().await {
        Ok(parameters) => parameters,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    if !args.no_metrics {
        if let Some(ref metrics) = parameters.metrics {
            let display_guard = datapipe::metrics::LiveDisplayGuard::start(
                metrics.clone(),
                std::time::Duration::from_millis(250),
            );
            let run_result = run_datapipe(parameters).await;
            display_guard.stop().await;
            match run_result {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("{}", err);
                    ExitCode::FAILURE
                }
            }
        } else {
            match run_datapipe(parameters).await {
                Ok(()) => ExitCode::SUCCESS,
                Err(err) => {
                    eprintln!("{}", err);
                    ExitCode::FAILURE
                }
            }
        }
    } else {
        match run_datapipe(parameters).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("{}", err);
                ExitCode::FAILURE
            }
        }
    }
}
