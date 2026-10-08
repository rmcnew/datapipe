//! Graphical user interface executable for datapipe.

use datapipe::logger::init_logger;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Install ring as rustls crypto provider
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        eprintln!("Failed to install ring as rustls crypto provider");
        return ExitCode::FAILURE;
    }

    // Initialize logger for background pipeline logging
    let _log_handle = match init_logger("/var/tmp", "datapipe-gui", false) {
        Ok(handle) => Some(handle),
        Err(err) => {
            eprintln!("Warning: Failed to initialize logger: {err}");
            None
        }
    };

    match datapipe::gui::run_gui() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error running datapipe GUI: {err}");
            ExitCode::FAILURE
        }
    }
}
