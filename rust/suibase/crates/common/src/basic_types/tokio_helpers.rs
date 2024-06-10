// Utilities that depends only on basic types, std and tokio.

use tokio::sync::mpsc::{Receiver, Sender};

use super::GenericChannelMsg;

pub fn remove_generic_event_dups(
    event_rx: &mut Receiver<GenericChannelMsg>,
    event_tx: &Sender<GenericChannelMsg>,
) {
    let mut seen_audit = false;
    let mut seen_update = false;
    let mut buffer = Vec::new();

    while let Ok(msg) = event_rx.try_recv() {
        match msg.event_id {
            super::EVENT_AUDIT if seen_audit => continue,
            super::EVENT_UPDATE if seen_update => continue,
            super::EVENT_AUDIT => seen_audit = true,
            super::EVENT_UPDATE => seen_update = true,
            _ => {}
        }
        buffer.push(msg);
    }

    for msg in buffer {
        let _ = event_tx.try_send(msg);
    }
}
