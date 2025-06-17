// various utilities
use crate::datapipe_types::DatapipeError;
use std::path::Path;
use std::string::ToString;
use tokio::net::UdpSocket;

#[tokio::test]
async fn test_get_unused_port() {
    let maybe_unused_port = get_unused_port().await;
    assert!(maybe_unused_port.is_some()); // highly unlikely that a machine is using all available ports
    let _unused_port = maybe_unused_port.unwrap();
    //println!("Found available port: {}", _unused_port);
}

/// see if a port is in use
pub async fn port_available(port: u16) -> bool {
    let addr = format!("127.0.0.1:{}", port);
    match UdpSocket::bind(addr).await {
        Ok(socket) => match socket.local_addr() {
            Ok(_address) => true,
            Err(_error) => false,
        },
        Err(_error) => false,
    }
}

const MAX_RETRY: u8 = 16;
const PORT_SEARCH_START: u16 = 42_000;
const PORT_SEARCH_END: u16 = 65_000; 

/// find an unused IP port
pub async fn get_unused_port() -> Option<u16> {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let mut rng = StdRng::from_os_rng();
    for _ in 0..MAX_RETRY {
        let port = rng.random_range(PORT_SEARCH_START..PORT_SEARCH_END);
        if port_available(port).await {
            return Some(port);
        }
    }
    None
}

/// sleep for a randomized amount up to 'max_delay_in_seconds' before continuing
pub async fn start_after_random_delay(max_delay_in_seconds: u8) {
    use rand::distr::Uniform;
    use rand::{rng, Rng};
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