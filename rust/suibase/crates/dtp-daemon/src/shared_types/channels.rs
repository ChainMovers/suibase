use common::basic_types::GenericChannelMsg;

pub type WebSocketWorkerIOTx = tokio::sync::mpsc::Sender<WebSocketWorkerIOMsg>;
pub type WebSocketWorkerIORx = tokio::sync::mpsc::Receiver<WebSocketWorkerIOMsg>;

// Enum for various message type that can be sent to this process.
// In particular, one type is a GenericChannelMessage
#[derive(Debug)]
pub enum WebSocketWorkerIOMsg {
    Generic(GenericChannelMsg),
    Extended(ExtendedWebSocketWorkerIOMsg),
}

// Add a few optional parameters to a GenericChannelMsg
#[derive(Default, Debug)]
pub struct ExtendedWebSocketWorkerIOMsg {
    pub generic: GenericChannelMsg,
    pub sender: Option<String>,  // Sui address of an auth sending data.
    pub package: Option<String>, // Sui address of the related DTP package (multiple publication can co-exists).
    pub localhost: Option<dtp_sdk::Host>,
    pub conn: Option<dtp_sdk::Connection>,
}

pub type WebSocketWorkerTx = tokio::sync::mpsc::Sender<WebSocketWorkerMsg>;
pub type WebSocketWorkerRx = tokio::sync::mpsc::Receiver<WebSocketWorkerMsg>;

// Enum for various message type that can be sent to this process.
// In particular, one type is a GenericChannelMessage
#[derive(Debug)]
pub enum WebSocketWorkerMsg {
    Generic(GenericChannelMsg),
    Extended(ExtendedWebSocketWorkerMsg),
}

// Add a few optional parameters to a GenericChannelMsg
#[derive(Default, Debug)]
pub struct ExtendedWebSocketWorkerMsg {
    pub generic: GenericChannelMsg,
    pub localhost: Option<dtp_sdk::Host>,
    pub remote_host: Option<dtp_sdk::Host>,
}
