// Must match Move definition(s) on network
pub type ServiceType = u8;

pub const C_SERVICE_TYPE_INVALID_IDX: u8 = 0;
pub const C_SERVICE_TYPE_INVALID_NAME: &str = "Invalid";
pub const C_SERVICE_TYPE_INVALID_PORT: u32 = 0;

// UDP Tunelling.
pub const C_SERVICE_TYPE_UDP_IDX: u8 = 1;
pub const C_SERVICE_TYPE_UDP_NAME: &str = "UDP";
pub const C_SERVICE_TYPE_UDP_PORT: u32 = 1;

// Remote Procedure Call (RPC)
pub const C_SERVICE_TYPE_JSON_RPC_2_0_IDX: u8 = 2;
pub const C_SERVICE_TYPE_JSON_RPC_2_0_NAME: &str = "JSON-RPC 2.0";
pub const C_SERVICE_TYPE_JSON_RPC_2_0_PORT: u32 = 2;

// GraphQL Service
pub const C_SERVICE_TYPE_GRAPHQL_IDX: u8 = 3;
pub const C_SERVICE_TYPE_GRAPHQL_NAME: &str = "GRAPHQL";
pub const C_SERVICE_TYPE_GRAPHQL_PORT: u32 = 3;

// HTTP (optionally encrypted)
pub const C_SERVICE_TYPE_HTTP_IDX: u8 = 4;
pub const C_SERVICE_TYPE_HTTP_NAME: &str = "HTTP";
pub const C_SERVICE_TYPE_HTTP_PORT: u32 = 80;

// HTTPS (always encrypted)
pub const C_SERVICE_TYPE_HTTPS_IDX: u8 = 5;
pub const C_SERVICE_TYPE_HTTPS_NAME: &str = "HTTPS";
pub const C_SERVICE_TYPE_HTTPS_PORT: u32 = 443;

// Ping (ICMP Echo Request/Reply)
pub const C_SERVICE_TYPE_ECHO_IDX: u8 = 7;
pub const C_SERVICE_TYPE_ECHO_NAME: &str = "ECHO";
pub const C_SERVICE_TYPE_ECHO_PORT: u32 = 7;

// gRPC
pub const C_SERVICE_TYPE_GRPC_IDX: u8 = 8;
pub const C_SERVICE_TYPE_GRPC_NAME: &str = "GRPC";
pub const C_SERVICE_TYPE_GRPC_PORT: u32 = 8;

// Discard Protocol
//
// Connection used to send any data. No guarantees of being process
// by the receiver. Data retention time is minimized (drop on network
// as soon as possible). Sender pays all costs.
//
// Intended for testing/benchmarking of sender.
pub const C_SERVICE_TYPE_DISCARD_IDX: u8 = 9;
pub const C_SERVICE_TYPE_DISCARD_NAME: &str = "DISCARD";
pub const C_SERVICE_TYPE_DISCARD_PORT: u32 = 9;

// [10..20] Available

// File Transfer
pub const C_SERVICE_TYPE_FTP_IDX: u8 = 21;
pub const C_SERVICE_TYPE_FTP_NAME: &str = "FTP";
pub const C_SERVICE_TYPE_FTP_PORT: u32 = 21;

// Secure Shell Protocol
pub const C_SERVICE_TYPE_SSH_IDX: u8 = 22;
pub const C_SERVICE_TYPE_SSH_NAME: &str = "SSH";
pub const C_SERVICE_TYPE_SSH_PORT: u32 = 22;

// !!! Update SERVICE_TYPE_MAX_IDX when appending new service types. !!!
pub const C_SERVICE_TYPE_MAX_IDX: u8 = 22;
