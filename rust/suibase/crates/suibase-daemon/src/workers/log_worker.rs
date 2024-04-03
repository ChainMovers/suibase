// Thread for writing and maintaining a log file.
//
// The logs are received through a channel.
//
// Lines are defined as:
//   [Time] [Level] "text" objects:[ {"type"="package_id::module::type", "address":"0x123~456", "object"={}}, ... ]
//
// 'text' can refer to an object with the label "$object:<address>"
//
// The log file will then have a JSON representation of these objects in an appended JSON array.
//
// The thread is auto-restart in case of panic.
use std::sync::Arc;

use crate::shared_types::{self, Globals, Workdir};

use common::basic_types::{
    self, AutoThread, GenericChannelMsg, GenericRx, GenericTx, Runnable, WorkdirIdx,
};

use axum::async_trait;

use tokio::sync::Mutex;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

// Schema: One entry per Package.

#[derive(Debug)]
struct SuiEvent {
    id: u64, // Sequence number within this table. Event assumed inserted in chronological order.
    package_instance_id: u64, // Foreign key into PackageInstance table.
    timestamp_ms: u64, // milliseconds. Also in results, but put here for sorting convenience.
    event_json: String, // This is the content of the "result" field (JSON object).
}

impl SuiEvent {
    pub fn new(package_instance_id: u64, timestamp_ms: u64, event_json: String) -> Self {
        Self {
            id: 0,
            package_instance_id,
            timestamp_ms,
            event_json,
        }
    }
}

#[derive(Clone)]
pub struct LogWorkerParams {
    globals: Globals,
    event_rx: Arc<Mutex<GenericRx>>,
    event_tx: GenericTx,
    workdir_idx: WorkdirIdx,
    workdir_name: String,
}

impl LogWorkerParams {
    pub fn new(
        globals: Globals,
        event_rx: GenericRx,
        event_tx: GenericTx,
        workdir_idx: WorkdirIdx,
        workdir_name: String,
    ) -> Self {
        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            event_tx,
            workdir_idx,
            workdir_name,
        }
    }
}

pub struct LogWorker {
    auto_thread: AutoThread<LogWorkerThread, LogWorkerParams>,
}

impl LogWorker {
    pub fn new(params: LogWorkerParams) -> Self {
        Self {
            auto_thread: AutoThread::new("DBWorker".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> anyhow::Result<()> {
        self.auto_thread.run(subsys).await
    }
}

#[derive(Debug, Default)]
struct LogManagement {
    schema_ok: bool, // Set when all tables have been verified to exist.
}

impl LogManagement {
    pub fn new() -> Self {
        Self { schema_ok: false }
    }
}

struct LogWorkerThread {
    thread_name: String,
    params: LogWorkerParams,
    log: Vec<LogManagement>,
    workdir: Workdir,
}

#[async_trait]
impl Runnable<LogWorkerParams> for LogWorkerThread {
    fn new(thread_name: String, params: LogWorkerParams) -> Self {
        Self {
            thread_name,
            params,
            log: Vec::new(),
            workdir: Default::default(),
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> anyhow::Result<()> {
        let output = format!("started for {}", self.params.workdir_name);
        log::info!("{}", output);

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("normal thread exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1)");
                Ok(())
            }
        }
    }
}

impl LogWorkerThread {
    async fn process_audit_msg(&mut self, msg: GenericChannelMsg) {
        if msg.event_id != basic_types::EVENT_AUDIT {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
            }
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
        }
    }

    async fn process_update_msg(&mut self, msg: GenericChannelMsg) {
        // Updates do nothing because not writing to globals yet.

        // Make sure the event_id is EVENT_UPDATE.
        if msg.event_id != basic_types::EVENT_UPDATE {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }

        // Verify that the workdir_idx is as expected.
        if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
            }
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
        }
    }

    async fn process_add_sui_event(&mut self, msg: GenericChannelMsg) {
        // Make sure the event is valid.
        if msg.event_id != basic_types::EVENT_EXEC {
            log::error!("Unexpected event_id {:?}", msg);
            return;
        }
        let _workdir_name = if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
            }
            shared_types::WORKDIRS_KEYS[workdir_idx as usize]
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
            return;
        };

        // Producer of add_sui_event should always set the Suibase uuid in msg.data_string.
        let _package_uuid = if let Some(package_uuid) = msg.params(0) {
            package_uuid
        } else {
            log::error!("Missing Suibase package UUID in params(0) {:?}", msg);
            return;
        };

        let _package_name = if let Some(package_name) = msg.params(1) {
            package_name
        } else {
            log::error!("Missing package name in params(1) {:?}", msg);
            return;
        };

        let data_json = if let Some(data_json) = msg.data_json {
            data_json
        } else {
            log::error!("Missing data_json {:?}", msg);
            return;
        };

        // Extract the "params" Object form data_json.
        let params_json =
            if let Some(params_json) = data_json.get("params").and_then(|v| v.as_object()) {
                params_json
            } else {
                log::error!("Missing params Object {:?}", data_json);
                return;
            };

        // Extract the "result" Object from params_json.

        let result_json =
            if let Some(result_json) = params_json.get("result").and_then(|v| v.as_object()) {
                result_json
            } else {
                log::error!("Missing result Object {:?}", data_json);
                return;
            };

        // Extract expected fields from the result_json
        //
        // Example of result_json
        // Object { "id": Object {"txDigest": String("3VuaCUx5K7bo7SCakPsFrVnoQzytvaVcYgcmVuftChrL"), "eventSeq": String("0")},
        //   "packageId": String("0xe0654f522ae3cb1a364174f740275d57f5a87b430d669c5a0554b975af683b08"),
        //   "transactionModule": String("Counter"),
        //   "sender": String("0xf7ae71f84fabc58662bd4209a8893f462c60f247095bb35b19ff659ad0081462"),
        //   "type": String("0xe0654f522ae3cb1a364174f740275d57f5a87b430d669c5a0554b975af683b08::console::ConsoleEvent"),
        //   "parsedJson": Object {"level": Number(3), "message": String("increment() entry called")},
        //   "bcs": String("6VrJC24y6KXNLbxK6HzfZWFwiEkJRrs"),
        //   "timestampMs": String("1703895010111")
        // }

        let package_id =
            if let Some(package_id) = result_json.get("packageId").and_then(|v| v.as_str()) {
                package_id
            } else {
                log::error!("Missing packageId {:?}", data_json);
                return;
            };

        // Verify there is an 0x prefix and remove it from package_id.
        if !package_id.starts_with("0x") {
            log::error!("Invalid packageId {:?}", data_json);
            return;
        }
        let _package_id = package_id[2..].to_string();

        let timestamp_ms: u64 =
            if let Some(timestamp_ms) = result_json.get("timestampMs").and_then(|v| v.as_str()) {
                timestamp_ms.parse::<u64>().unwrap_or(0)
            } else {
                log::error!("Missing timestampMs {:?}", data_json);
                return;
            };

        if timestamp_ms == 0 {
            log::error!("Invalid timestampMs {:?}", data_json);
            return;
        };

        let type_str = if let Some(type_str) = result_json.get("type").and_then(|v| v.as_str()) {
            type_str
        } else {
            log::error!("Missing type {:?}", data_json);
            return;
        };

        let (is_console, sub_table_name) = if type_str.ends_with("::ConsoleEvent") {
            (true, "console")
        } else {
            (false, "user")
        };

        let (event_level, event_message, event_sender) = if !is_console {
            (0u8, "", None)
        } else {
            //   "parsedJson": Object {"level": Number(3), "message": String("X"), "sender": String("0x...")},

            let parsed_json = if let Some(parsed_json) =
                result_json.get("parsedJson").and_then(|v| v.as_object())
            {
                parsed_json
            } else {
                log::error!("Missing parsedJson {:?}", data_json);
                return;
            };

            let event_level =
                if let Some(event_level) = parsed_json.get("level").and_then(|v| v.as_u64()) {
                    if event_level > basic_types::EVENT_LEVEL_MAX as u64 {
                        log::error!("Invalid above MAX parsedJson.level {:?}", data_json);
                        return;
                    }
                    if event_level < basic_types::EVENT_LEVEL_MIN as u64 {
                        log::error!("Invalid below MIN parsedJson.level  {:?}", data_json);
                        return;
                    }
                    event_level as u8
                } else {
                    log::error!("Missing parsedJson.level {:?}", data_json);
                    return;
                };
            let event_message = parsed_json.get("message").and_then(|v| v.as_str());
            if event_message.is_none() {
                log::error!("Missing parsedJson.message {:?}", data_json);
                return;
            };

            let event_message = event_message.unwrap();
            let event_sender = parsed_json.get("sender").and_then(|v| v.as_str());
            // log::info!("parsed_json {:?}", parsed_json);

            (event_level, event_message, event_sender)
        };

        // Append event into log (should be already created).
        let _name_suffix = format!("{}_{}", sub_table_name, event_level);
        let event_json = if is_console {
            serde_json::json!({
                "sender": event_sender,
                "message": event_message,
            })
        } else {
            let res = serde_json::to_string(&result_json);
            let message = if let Ok(res) = res {
                res
            } else {
                format!("Failed to stringify result_json {:?}", result_json)
            };
            // TODO Find a way to resolve the sender.
            serde_json::json!({
                "sender": "unknown",
                "message": message,
            })
        };
        // Stringify event_json and insert it in DB.
        let event_json = serde_json::to_string(&event_json);
        if event_json.is_err() {
            log::error!("Failed to stringify event_json {:?}", event_json);
            return;
        }
        let event_json = event_json.unwrap();
        // Make sure even_json is safe by escaping all double quotes with double-double quotes (SQLite way).
        let _event_json = event_json.replace('\"', "\"\"");
        // let mut new_sui_event = SuiEvent::new(package_instance.id, timestamp_ms, event_json);
        /*if let Err(e) =
            package_instance.insert_event_in_db(conn, &package, name_suffix, &mut new_sui_event)
        {
            log::error!("Failed to insert SuiEvent in DB {:?}", e);
        }*/

        // TODO Broadcast the sequence number increment of this sui_event object to websocket users.
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // Take mutable ownership of the event_rx channel as long this thread is running.
        let event_rx = Arc::clone(&self.params.event_rx);
        let mut event_rx = event_rx.lock().await;

        // Open the database connection.
        /*
        if self.db.conn.is_none() && !self.open_db().await {
            // Delay of 5 seconds before retrying.
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            return;
        }*/

        while !subsys.is_shutdown_requested() {
            let event_rx_future = futures::FutureExt::fuse(event_rx.recv());

            tokio::select! {
                /*msg = ws_stream_future => {
                    if let Some(msg) = msg {
                        let msg = msg.unwrap();
                        self.process_ws_msg(msg).await;
                    } else {
                        // Shutdown requested.
                        log::info!("Received a None websocket message");
                        return;
                    }
                }*/
                msg = event_rx_future => {
                    if let Some(msg) = msg {
                        // Process the message.
                        match msg.event_id {
                            basic_types::EVENT_AUDIT => {
                                self.process_audit_msg(msg).await;
                            },
                            basic_types::EVENT_UPDATE => {
                                self.process_update_msg(msg).await;
                            },
                            basic_types::EVENT_EXEC => {
                                if let Some(command) = msg.command() {
                                    if command == "add_sui_event" {
                                        self.process_add_sui_event(msg).await;
                                    } else {
                                        log::error!("Received a EVENT_EXEC message with unexpected command {}", command);
                                    }
                                } else {
                                    log::error!("Received a EVENT_EXEC message without command");
                                }
                            },
                            _ => {
                                // Consume unexpected messages.
                                log::error!("Unexpected event_id {:?}", msg );
                            }
                        }
                    } else {
                        // Channel closed or shutdown requested.
                        log::info!("Received a None internal message");
                        return;
                    }
                }
            }
        }
    }
}
