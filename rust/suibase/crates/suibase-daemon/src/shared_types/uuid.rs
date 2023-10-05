// MTSafeUUID provides:
//   - a get() function that returns both a server_id (UUID v4) and a data_version (UUID v7).
//   - The server_id is initialized once on process startup and changes whenever
//     a data_version is unexpectedly not higher than the previous one generated.
//   - get() is multi-thread safe (Mutex protected).
//
// SingleThreadUUID is same, except the user is responsible for Mutex access.
//
use std::sync::{Arc, Mutex};
use uuid::{Uuid, Variant, Version};
use uuid7::{uuid7, V7Generator};

#[cfg(not(test))]
use log::{info, warn};

#[cfg(test)]
use std::{println as info, println as warn};

#[derive(Clone, Debug)]
pub struct SingleThreadUUID {
    server_id: Uuid,
    data_version: Uuid,
}

impl SingleThreadUUID {
    pub fn new() -> Self {
        Self {
            server_id: Uuid::new_v4(),
            data_version: uuid7::new_v7(),
        }
    }

    pub fn get(&self) -> (Uuid, Uuid) {
        (self.server_id, self.data_version)
    }

    pub fn set(&mut self, other: &Self) {
        self.server_id = other.server_id;
        self.data_version = other.data_version;
    }

    pub fn increment(&mut self) {
        let new_data_version: Uuid = uuid7::new_v7();

        //info!("data_version: {}", new_data_version.to_string());
        //info!("server_id: {}", self.server_id.to_string());
        /*
        if let Some(version) = new_data_version.get_version() {
          if version != Version::SortRand {
            warn!("WARNING: UUID data_version is not random");
          }
        }*/
        if new_data_version <= self.data_version {
            self.server_id = Uuid::new_v4();
        }
        self.data_version = new_data_version;
    }
}

impl PartialEq for SingleThreadUUID {
    fn eq(&self, other: &Self) -> bool {
        self.server_id == other.server_id && self.data_version == other.data_version
    }
}

pub type MTSafeUUID = Arc<tokio::sync::Mutex<SingleThreadUUID>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_one_thread_uuid() {
        let single_thread_uuid = SingleThreadUUID::new();
        let mt_safe_uuid = Arc::new(tokio::sync::Mutex::new(single_thread_uuid));
        let mut locked_uuid = mt_safe_uuid.lock().await;
        let mut prev_data_version = locked_uuid.data_version;
        let initial_server_id = locked_uuid.server_id;
        for _ in 0..100000 {
            locked_uuid.increment();
            let (server_id, data_version) = locked_uuid.get();

            assert_eq!(server_id, initial_server_id);
            assert!(data_version > prev_data_version);

            prev_data_version = data_version;
        }
    }

    #[tokio::test]
    async fn test_two_threads_uuid() {
        let single_thread_uuid = SingleThreadUUID::new();
        let mt_safe_uuid = Arc::new(tokio::sync::Mutex::new(single_thread_uuid));
        let mt_safe_uuid_clone = mt_safe_uuid.clone();

        let (initial_server_id, mut prev_data_version) = {
            let locked_uuid = mt_safe_uuid.lock().await;
            (locked_uuid.server_id, locked_uuid.data_version)
        };

        let (_result1, _result2) = tokio::join!(
            async move {
                for _ in 0..100000 {
                    let mut locked_uuid = mt_safe_uuid.lock().await;
                    locked_uuid.increment();
                    let (server_id, data_version) = locked_uuid.get();

                    assert_eq!(server_id, initial_server_id);
                    assert!(data_version > prev_data_version);

                    prev_data_version = data_version;
                }
            },
            async move {
                for _ in 0..100000 {
                    let mut locked_uuid = mt_safe_uuid_clone.lock().await;
                    locked_uuid.increment();
                    let (server_id, data_version) = locked_uuid.get();

                    assert_eq!(server_id, initial_server_id);
                    assert!(data_version > prev_data_version);

                    prev_data_version = data_version;
                }
            }
        );
    }
}
