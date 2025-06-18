// various utilities
use crate::datapipe_types::DatapipeError;
use rand::distr::Uniform;
use rand::{rng, Rng};
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

/// see if a port is in use; used in testing, so minimal error handling
pub fn port_available(port: u16) -> bool {
    let result: bool;
    { // use an explicit scope so the socket is dropped
        let addr = format!("127.0.0.1:{}", port).parse().unwrap();
        let socket = TcpSocket::new_v4().unwrap();
        // set socket options so the port is available as soon as possible
        socket.set_reuseaddr(true).unwrap();
        socket.set_reuseport(true).unwrap();
        socket.set_keepalive(false).unwrap();
        socket.set_linger(None).unwrap();
        match socket.bind(addr) {
            Ok(()) => match socket.local_addr() {
                Ok(_address) => {
                    result = true;
                }
                Err(_error) => {
                    result = false;
                }
            },
            Err(_error) => {
                result = false;
            }
        }
    }
    result
}

const MAX_RETRY: u8 = 16;
const PORT_SEARCH_START: u16 = 24_000;
const PORT_SEARCH_END: u16 = 64_000; 

/// find an unused IP port
pub async fn get_unused_port() -> Option<u16> {
    let range = Uniform::try_from(PORT_SEARCH_START..PORT_SEARCH_END).unwrap();
    for _ in 0..MAX_RETRY {
        let port = rng().sample(range);
        if port_available(port) {
            // short wait to allow port to be available; 
            // on some systems, it takes a bit longer for the socket to be ready for use
            // and trying to use it immediately gives a 'port already in use' error
            tokio::time::sleep(std::time::Duration::from_millis(750)).await;
            return Some(port);
        }
    }
    None
}

/// sleep for a randomized amount up to 'max_delay_in_seconds' before continuing
pub async fn start_after_random_delay(max_delay_in_seconds: u8) {
    let range = Uniform::try_from(1..max_delay_in_seconds).unwrap();
    let delay = rng().sample(range);
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
        Err(error) => {
            match error {
                std::env::VarError::NotPresent => {
                    match std::env::var("COMPUTERNAME") {
                        Ok(hostname) => hostname,
                        Err(_error) => "localhost".to_string()
                    }
                }
                _ => {
                    "localhost".to_string()
                }
            }
        }
    }
}

/// compare file contents
pub async fn identical_contents(file1: impl AsRef<Path>, file2: impl AsRef<Path>) -> Result<bool, DatapipeError> {
    let contents1 = tokio::fs::read(file1).await?;
    let contents2 = tokio::fs::read(file2).await?;
    Ok(contents1 == contents2)
}
