use crate::basic_types::*;
use std::net::SocketAddr;

pub struct TargetServer {
    pub address: SocketAddr,
    pub healthy: bool,
    pub last_healthy: EpochTimestamp,
}

impl TargetServer {
    pub fn new(connection_string: &str) -> Self {
        // TODO Re-write this to make it robust... can't panic here.
        // Build a SocketAddr from a String. Parse the IP, the port and ignore the protocol.
        let mut int_connection_string = connection_string.to_string();
        let mut _protocol = "http://".to_string();
        if int_connection_string.starts_with("http://") {
            int_connection_string = int_connection_string.replacen("http://", "", 1);
        } else if int_connection_string.starts_with("https://") {
            int_connection_string = int_connection_string.replacen("https://", "", 1);
            _protocol = "https://".to_string();
        }
        let mut connection_string_split = int_connection_string.split(":");
        let ip = connection_string_split.next().unwrap();
        let port = connection_string_split.next().unwrap();
        let address = format!("{}:{}", ip, port);
        let address = address.parse::<SocketAddr>().unwrap();

        Self {
            address,
            healthy: false,
            last_healthy: EpochTimestamp::now(),
        }
    }
}
