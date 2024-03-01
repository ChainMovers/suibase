use crate::basic_types::AutoSizeVec;

#[derive(Debug, Clone)]
pub struct SuiEventData {
    pub msg: String,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct EventsWorkdirData {
    pub user_events: Vec<SuiEventData>,
    pub console_events: Vec<SuiEventData>,
    pub watch_events: Vec<SuiEventData>,
}

impl EventsWorkdirData {
    pub fn new() -> Self {
        Self {
            user_events: Vec::new(),
            console_events: Vec::new(),
            watch_events: Vec::new(),
        }
    }
}

impl std::default::Default for EventsWorkdirData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GlobalsEventsDataST {
    // One per workdir, WorkdirIdx maintained by workdirs.
    pub workdirs: AutoSizeVec<EventsWorkdirData>,
}

impl GlobalsEventsDataST {
    pub fn new() -> Self {
        Self {
            workdirs: AutoSizeVec::new(),
        }
    }
}

impl Default for GlobalsEventsDataST {
    fn default() -> Self {
        Self::new()
    }
}
