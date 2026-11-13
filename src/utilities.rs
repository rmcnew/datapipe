// various utilities
use crate::datapipe_types::DatapipeError;
use rand::distr::Uniform;
use rand::{RngExt, rng};
use std::path::Path;
use std::string::ToString;
use tokio::net::TcpSocket;

#[tokio::test]
async fn test_get_unused_port() {
    let maybe_unused_port = get_unused_port().await;
    assert!(maybe_unused_port.is_some()); // highly unlikely that a machine is using all available ports
    let _unused_port = maybe_unused_port.unwrap();
    //println!("Found available port: {}", _unused_port);
}

/// Check whether a local TCP port can be bound on `127.0.0.1`.
pub fn port_available(port: u16) -> bool {
    let Ok(addr) = format!("127.0.0.1:{port}").parse() else {
        return false;
    };
    let Ok(socket) = TcpSocket::new_v4() else {
        return false;
    };
    if socket.set_reuseaddr(true).is_err() {
        return false;
    }
    #[cfg(not(windows))]
    if socket.set_reuseport(true).is_err() {
        return false;
    }
    if socket.set_keepalive(false).is_err() {
        return false;
    }
    socket.bind(addr).is_ok() && socket.local_addr().is_ok()
}

const MAX_RETRY: u8 = 16;
const PORT_SEARCH_START: u16 = 24_000;
const PORT_SEARCH_END: u16 = 64_000;

/// Find an unused IP port.
pub async fn get_unused_port() -> Option<u16> {
    let Ok(range) = Uniform::try_from(PORT_SEARCH_START..PORT_SEARCH_END) else {
        return None;
    };
    for _ in 0..MAX_RETRY {
        let port = rng().sample(range);
        if port_available(port) {
            // Short wait to allow the socket to settle cleanly on all platforms.
            tokio::time::sleep(std::time::Duration::from_millis(750)).await;
            return Some(port);
        }
    }
    None
}

/// Sleep for a randomized duration in seconds up to `max_delay_in_seconds`.
pub async fn start_after_random_delay(max_delay_in_seconds: u8) {
    if max_delay_in_seconds == 0 {
        return;
    }
    let delay = match Uniform::try_from(1..=max_delay_in_seconds) {
        Ok(range) => rng().sample(range),
        Err(_) => max_delay_in_seconds,
    };
    tokio::time::sleep(std::time::Duration::from_secs(delay.into())).await;
}

#[test]
fn test_utilities_misc_hostname() {
    let name = hostname();
    println!("hostname is {name}");
    assert!(!name.is_empty());
}

/// get the system hostname
pub fn hostname() -> String {
    match std::env::var("HOSTNAME") {
        Ok(hostname) => hostname,
        Err(error) => match error {
            std::env::VarError::NotPresent => match std::env::var("COMPUTERNAME") {
                Ok(hostname) => hostname,
                Err(_error) => "localhost".to_string(),
            },
            _ => "localhost".to_string(),
        },
    }
}

/// compare file contents
pub async fn identical_contents(
    file1: impl AsRef<Path>,
    file2: impl AsRef<Path>,
) -> Result<bool, DatapipeError> {
    let contents1 = tokio::fs::read(file1).await?;
    let contents2 = tokio::fs::read(file2).await?;
    Ok(contents1 == contents2)
}
