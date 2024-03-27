use std::collections::HashMap;

// Globals related to the processing of data received from DTP connections.
//
// In particular, keep tracks of all one-shot channels for pending requests.

// State machine used by websocket_worker to track package subscription with a single server.
#[derive(Debug, Clone, PartialEq)]
pub enum RequestTrackingState {
    // States when all goes well
    Created, // A thread created the request, but no other thread have yet do anything about it.
    Requested, // The request was handled by a thread and succeeded. Next state update will be about the response.
    ResponseReceived, // The response was received with success.

    // Failure scenario
    FailedRequest, // The request was attempted, but failed. It will not be further attempted.
    FailedRequestTimeout, // The request was not sent in time.
    FailedResponseTimeout, // The response was not received in time.
    FailedResponse, // The response was received but ill-formed.

                   // The following state will fire the oneshot response channel:
                   //   - ResponseReceived.
                   //   - Any of the "Failed" states.
                   //
                   // The one-shot response is a stringify JSON Object that always start the "state" field:
                   //    { "state": "ResponseReceived", ... }
                   //
                   // More fields may be added depending of the request and its outcome.
}

impl Default for RequestTrackingState {
    fn default() -> Self {
        RequestTrackingState::Created
    }
}

impl RequestTrackingState {
    pub fn to_string(&self) -> String {
        match self {
            RequestTrackingState::Created => "Created",
            RequestTrackingState::Requested => "Requested",
            RequestTrackingState::ResponseReceived => "ResponseReceived",
            RequestTrackingState::FailedRequest => "FailedRequest",
            RequestTrackingState::FailedRequestTimeout => "FailedRequestTimeout",
            RequestTrackingState::FailedResponseTimeout => "FailedResponseTimeout",
            RequestTrackingState::FailedResponse => "FailedResponse",
        }
        .to_string()
    }
    pub fn to_json(&self) -> String {
        format!(r#""state": "{}""#, self.to_string())
    }
}

#[derive(Debug)]
pub struct PendingResponse {
    state: RequestTrackingState,
    resp_channel: Option<tokio::sync::oneshot::Sender<String>>,
}
// Use the seq_num as key.
pub type PendingResponsesMap = HashMap<u64, PendingResponse>;

#[derive(Debug)]

pub struct GlobalsDTPConnsStateTxST {
    // A connection can have multiple pending requests.
    //
    // Uses the TransportControl address for key ("0x" string).
    pub pending_resp: HashMap<String, PendingResponsesMap>,
}

impl GlobalsDTPConnsStateTxST {
    pub fn new() -> Self {
        Self {
            pending_resp: HashMap::new(),
        }
    }
}

impl std::default::Default for GlobalsDTPConnsStateTxST {
    fn default() -> Self {
        Self::new()
    }
}
