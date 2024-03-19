// State machine used by websocket_worker to track package subscription with a single server.
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionTrackingState {
    // Valid state transitions:
    //  - *start state* -> Disconnected
    //  - Disconnected  -> Subscribing, ReadyToDelete
    //  - Subscribing   -> Subscribed, Disconnected, ReadyToDelete
    //  - Subscribed    -> Unsubscribing, Disconnected, ReadyToDelete
    //  - Unsubscribing -> ReadyToDelete
    //  - ReadyToDelete   -> *end state*
    //
    // Notable logic:
    //  connection open and package in config  -> Subscribing
    //  connection close and package in config -> Disconnected
    //  package removed from config and Subscribed -> Unsubscribing
    //  package removed from config and not Subscribed -> ReadyToDelete
    Disconnected,  // Initial state or closed connection.
    Subscribing,   // Trying to subscribe, not confirm yet.
    Subscribed,    // Confirmed subscribed and connection is open.
    Unsubscribing, // Opened connection, trying to unsubscribe.
    ReadyToDelete, // No longer needed, unsubscription was confirmed (or timeout)
}

impl SubscriptionTrackingState {
    pub fn new() -> Self {
        Self::Disconnected
    }
}

impl Default for SubscriptionTrackingState {
    fn default() -> Self {
        Self::new()
    }
}

impl From<SubscriptionTrackingState> for u32 {
    fn from(val: SubscriptionTrackingState) -> Self {
        match val {
            SubscriptionTrackingState::Disconnected => 0,
            SubscriptionTrackingState::Subscribing => 1,
            SubscriptionTrackingState::Subscribed => 2,
            SubscriptionTrackingState::Unsubscribing => 3,
            SubscriptionTrackingState::ReadyToDelete => 4,
        }
    }
}

#[derive(Debug, Default)]
pub struct SubscriptionTracking {
    // Set once on instantiation.
    toml_path: String,
    name: String,
    uuid: String,
    id: String,
    is_package: bool,

    // State machine.
    state: SubscriptionTrackingState,

    // Time of last state change (init to creation time).
    state_change_timestamp: Option<tokio::time::Instant>,

    // Stats on requests sent.
    request_sent_timestamp: Option<tokio::time::Instant>,
    request_retry: u8,

    // Set when a subscription response is received. This is then
    // used later to do event correlation and unsubscribe.
    unsubscribed_id: Option<String>,

    // This is a cached u64 conversion of unsubscribed_id.
    // Initialize to u64::MAX and updated whenever unsubscribed_id is set
    // with a valid number.
    subscription_number: u64,

    // sequence numbers that were used for
    // subscription request(s).
    subscribe_seq_numbers: Vec<u64>,

    // sequence numbers that were used for
    // un-subscription request(s).
    unsubscribe_seq_numbers: Vec<u64>,

    // Once requested to be removed from config, there is no way to go back.
    remove_request: bool,
}

impl SubscriptionTracking {
    pub fn new_for_package(toml_path: String, name: String, uuid: String, id: String) -> Self {
        let now = tokio::time::Instant::now();
        Self {
            toml_path,
            name,
            uuid,
            is_package: true,
            id,
            state: SubscriptionTrackingState::Disconnected,
            state_change_timestamp: Some(now),
            request_sent_timestamp: None,
            request_retry: 0,
            unsubscribed_id: None,
            subscription_number: u64::MAX,
            subscribe_seq_numbers: Vec::new(),
            unsubscribe_seq_numbers: Vec::new(),
            remove_request: false,
        }
    }

    pub fn new_for_object(id: String) -> Self {
        let now = tokio::time::Instant::now();
        Self {
            toml_path: String::new(),
            name: String::new(),
            uuid: String::new(),
            is_package: false,
            id,
            state: SubscriptionTrackingState::Disconnected,
            state_change_timestamp: Some(now),
            request_sent_timestamp: None,
            request_retry: 0,
            unsubscribed_id: None,
            subscription_number: u64::MAX,
            subscribe_seq_numbers: Vec::new(),
            unsubscribe_seq_numbers: Vec::new(),
            remove_request: false,
        }
    }

    pub fn toml_path(&self) -> &String {
        &self.toml_path
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn state(&self) -> &SubscriptionTrackingState {
        &self.state
    }

    pub fn uuid(&self) -> &String {
        &self.uuid
    }

    pub fn is_package(&self) -> bool {
        self.is_package
    }

    pub fn id(&self) -> &String {
        &self.id // That is the hexadecimal package id.
    }

    pub fn request_retry(&self) -> u8 {
        self.request_retry
    }

    pub fn unsubscribed_id(&self) -> Option<&String> {
        self.unsubscribed_id.as_ref()
    }

    pub fn subscription_number(&self) -> u64 {
        self.subscription_number
    }

    pub fn did_sent_subscribe_request(&self, seq_number: u64) -> bool {
        self.subscribe_seq_numbers.contains(&seq_number)
    }

    pub fn did_sent_unsubscribe_request(&self, seq_number: u64) -> bool {
        self.unsubscribe_seq_numbers.contains(&seq_number)
    }

    pub fn can_be_deleted(&self) -> bool {
        match self.state {
            SubscriptionTrackingState::Disconnected | SubscriptionTrackingState::ReadyToDelete => {
                // If the package is not subscribed (or about to), it can be deleted.
                true
            }
            SubscriptionTrackingState::Subscribing | SubscriptionTrackingState::Subscribed => {
                // If the package tracking is trying to subscribe (or already subscribed),
                // it cannot immediately be deleted.
                //
                // Must first wait for response or timeout of potential pending request
                // and then transition to Unsubscribing. This will lead to an
                // eventual ReadyToDelete.
                false
            }
            SubscriptionTrackingState::Unsubscribing => {
                // If the package is trying to unsubscribe, it cannot be deleted
                // until response or timeout.
                false
            }
        }
    }

    pub fn secs_since_last_request(&self) -> u64 {
        match self.request_sent_timestamp {
            Some(timestamp) => timestamp.elapsed().as_secs(),
            None => u64::MAX,
        }
    }

    pub fn is_subscribe_request_pending_response(&self) -> bool {
        self.state == SubscriptionTrackingState::Subscribing
            && self.request_sent_timestamp.is_some()
            && self.unsubscribed_id.is_none()
    }

    pub fn is_remove_requested(&self) -> bool {
        self.remove_request
    }

    pub fn change_state_to(&mut self, new_state: SubscriptionTrackingState) -> bool {
        if self.state == new_state {
            return false; // Nothing to do.
        }
        log::info!(
            "package_tracking: state change {:?} -> {:?} for package id 0x{}",
            self.state,
            new_state,
            self.id
        );
        if new_state == SubscriptionTrackingState::Disconnected {
            self.request_sent_timestamp = None;
            self.subscribe_seq_numbers.clear();
            self.unsubscribe_seq_numbers.clear();
            self.unsubscribed_id = None;
            self.subscription_number = u64::MAX;
            self.request_retry = 0;
        }
        self.state_change_timestamp = Some(tokio::time::Instant::now());
        self.state = new_state;
        true
    }

    // Various way to report external actions/events.
    pub fn report_subscribing_request(&mut self, seq_number: u64) {
        self.subscribe_seq_numbers.push(seq_number);
        self.request_sent_timestamp = Some(tokio::time::Instant::now());
        self.request_retry += 1;
        // Remove oldest request to avoid "memory leak" when failing for a long time.
        if self.subscribe_seq_numbers.len() > 50 {
            self.subscribe_seq_numbers.remove(0);
        }
    }

    pub fn report_subscribing_response(&mut self, unsubscribe_id: String) {
        self.request_sent_timestamp = None;
        self.request_retry = 0;
        // Convert unsubscribed_id to subscription_number (u64). The number is
        // an integer base 10. If fails, to convert, then set to u64::MAX.
        self.subscription_number = match unsubscribe_id.parse() {
            Ok(number) => number,
            Err(_e) => u64::MAX,
        };
        self.unsubscribed_id = Some(unsubscribe_id);
    }

    pub fn report_unsubscribing_request(&mut self, seq_number: u64) {
        self.unsubscribe_seq_numbers.push(seq_number);
        self.request_sent_timestamp = Some(tokio::time::Instant::now());
        self.request_retry += 1;
        // Remove oldest request to avoid "memory leak" when failing for a long time.
        if self.unsubscribe_seq_numbers.len() > 50 {
            self.unsubscribe_seq_numbers.remove(0);
        }
    }
    pub fn report_unsubscribing_response(&mut self) {
        self.unsubscribe_seq_numbers.clear();
        self.request_sent_timestamp = None;
        self.request_retry = 0;
        self.unsubscribed_id = None;
        self.subscription_number = u64::MAX;
    }

    pub fn report_remove_request(&mut self) {
        self.remove_request = true; // Once set, can never be cleared.
    }
}
