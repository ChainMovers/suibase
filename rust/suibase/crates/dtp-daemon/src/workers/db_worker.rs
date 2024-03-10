// Child thread of events_writer_worker
//
// Responsible to:
//   - Create and maintain the ~/suibase/workdirs/{workdir}/indexer/sqlite.db file.
//   - Receive validated and dedup data from parent events_writer_worker and
//     is responsible to perform and "confirm" the writing. Particularly, if the writing
//     is not successful (e.g. panic), then the data remains in the queue.
//
// The thread is auto-restart in case of panic.

use std::sync::Arc;

use crate::shared_types::{self, Globals};
use common::shared_types::{GlobalsWorkdirsST, Workdir};

use common::basic_types::{
    self, AutoThread, DBTable, GenericChannelMsg, GenericRx, GenericTx, Runnable, WorkdirIdx,
};

use rusqlite::Connection;

use axum::async_trait;

use tokio::sync::Mutex;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

// Schema: One entry per Package.

fn sanitize_table_name(name: String) -> String {
    name.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => c,
            _ => '_',
        })
        .collect()
}

#[derive(Clone, Debug)]
struct Package {
    id: u64, // This is the DB id. The network package_id is stored in one of the child PackageInstance.
    package_uuid: String,
    package_name: String,
    latest_instance_id: Option<u64>, // Foreign key into one of the PackageInstance child.
    table_prefix: String,            // {namespace}_{workdir_name}
    table_fullname: String,          // {namespace}_{workdir_name}_instance
}

impl DBTable for Package {
    fn create_table(
        conn: &Connection,
        workdir_name: String,
        namespace: Option<String>,
        _name_suffix: Option<String>,
    ) -> rusqlite::Result<()> {
        // TODO Recreate the table if not the latest schema version.
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {0}_{1}_package (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                package_uuid    TEXT NOT NULL,
                package_name    TEXT NOT NULL,                
                latest_instance_id INTEGER REFERENCES {0}_{1}_package_instance (id)
            )",
            workdir_name,
            namespace.unwrap_or_else(|| "sui".to_string()),
        );
        conn.execute(&sql, [])?;
        Ok(())
    }
}

impl Package {
    pub fn get_objs_from_db(
        conn: &Connection,
        workdir_name: String,
        namespace: Option<String>,
        package_uuid: String,
        package_name: String,
        package_id: String,
    ) -> Option<Box<(Package, PackageInstance)>> {
        let namespace = namespace.unwrap_or_else(|| "sui".to_string());

        // Get the Package from the DB (will be created as needed).
        let mut package = Self::get_package_from_db(
            conn,
            workdir_name.clone(),
            namespace.clone(),
            package_uuid.clone(),
            package_name.clone(),
        );

        if let Ok(package) = package.as_mut() {
            // Create a PackageInstance as needed.
            // When created, this become the "latest" instance.
            let package_instance = package.get_package_instance_from_db(conn, package_id.clone());
            if let Err(e) = package_instance {
                log::error!(
                    "Failed to create PackageInstance in DB for {} {} {} {} {:?}",
                    workdir_name,
                    namespace,
                    package_uuid,
                    package_name,
                    e
                );
                return None;
            }

            let package_instance = package_instance.as_ref().unwrap();

            let latest_instance_id: Option<u64> = package.latest_instance_id;
            if latest_instance_id.is_none() || (latest_instance_id.unwrap() != package_instance.id)
            {
                package.latest_instance_id = Some(package_instance.id);
                if let Err(e) = package.update_latest_instance_id_in_db(conn, package_instance.id) {
                    log::error!("Failed to update Package latest_instance_id: {:?}", e);
                    return None;
                }
            }
            // TODO Find a way to eliminate clone here! (which was the intent for using Box<>).
            return Some(Box::new((package.clone(), package_instance.clone())));
        }

        log::error!(
            "Failed to create package in DB for {} {} {} {}",
            workdir_name,
            namespace,
            package_uuid,
            package_name
        );
        None
    }

    fn get_package_instance_from_db(
        &self,
        conn: &Connection,
        package_id: String,
    ) -> rusqlite::Result<PackageInstance> {
        let table_name = format!("{}_instance", self.table_fullname);
        let sql = format!(
            "SELECT id
            FROM {}
            WHERE package_id = \"{}\" AND parent_id = {}",
            table_name, package_id, self.id
        );
        // log::info!("DOING SQL: {}", sql);
        let mut stmt = conn.prepare(&sql);
        if let Ok(stmt) = stmt.as_mut() {
            let mut rows = stmt.query([]);
            if let Ok(rows) = rows.as_mut() {
                let row = rows.next()?;
                if let Some(row) = row {
                    let id: u64 = row.get(0)?;
                    return Ok(PackageInstance {
                        id,
                        parent_id: self.id,
                        package_id,
                    });
                } else {
                    // Default initialization.
                    let mut new_row = PackageInstance {
                        id: 0,
                        parent_id: self.id,
                        package_id: package_id.clone(),
                    };
                    if let Err(e) = new_row.insert_in_db(conn, self) {
                        log::error!("Failed to insert new Package instance in {} for parent_id={} and package_id={} row: {:?}", 
                            table_name, self.id, package_id, e);
                        return Err(e);
                    } else {
                        return Ok(new_row);
                    }
                }
            }
        } else {
            log::error!("Failed to prepare SQL: {}", sql);
            return Err(rusqlite::Error::InvalidQuery);
        }

        log::error!("Failed to query SQL: {}", sql);
        Err(rusqlite::Error::InvalidQuery)
    }

    fn get_package_from_db(
        conn: &Connection,
        workdir_name: String,
        namespace: String,
        package_uuid: String,
        package_name: String,
    ) -> rusqlite::Result<Package> {
        let table_prefix = format!("{}_{}", workdir_name, namespace);
        let table_fullname = format!("{}_package", table_prefix);
        let sql = format!(
            "SELECT id, package_uuid, package_name, latest_instance_id
            FROM {}
            WHERE package_uuid = ?1",
            table_fullname
        );
        // log::info!("DOING SQL: {}", sql);
        let mut stmt = conn.prepare(&sql);
        if let Ok(stmt) = stmt.as_mut() {
            let rows = stmt.query([package_uuid.clone()]);
            if let Ok(mut rows) = rows {
                let row = rows.next()?;
                if let Some(row) = row {
                    let id: u64 = row.get(0)?;
                    let package_uuid: String = row.get(1)?;
                    let package_name: String = row.get(2)?;
                    let latest_instance_id: Option<u64> = row.get(3)?;
                    return Ok(Package {
                        id,
                        package_uuid,
                        package_name,
                        latest_instance_id,
                        table_prefix,
                        table_fullname,
                    });
                } else {
                    // Row Insertion
                    let mut new_row = Self {
                        id: 0,
                        package_uuid,
                        package_name,
                        latest_instance_id: None,
                        table_prefix,
                        table_fullname,
                    };
                    if let Err(e) = new_row.insert_in_db(conn) {
                        log::error!("Failed to insert new Package row: {:?}", e);
                        return Err(e);
                    } else {
                        return Ok(new_row);
                    }
                }
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows)
    }

    fn insert_in_db(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        let sql = if let Some(latest_instance_id) = self.latest_instance_id {
            format!(
                "INSERT INTO {0} (package_uuid, package_name, latest_instance_id)
            VALUES (\"{1}\", \"{2}\", {3}) RETURNING id",
                self.table_fullname,
                self.package_uuid.clone(),
                self.package_name.clone(),
                latest_instance_id
            )
        } else {
            format!(
                "INSERT INTO {0} (package_uuid, package_name)
            VALUES (\"{1}\", \"{2}\") RETURNING id",
                self.table_fullname,
                self.package_uuid.clone(),
                self.package_name.clone()
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        self.id = stmt.query_row([], |row| row.get(0))?;

        // Runtime test (Remove eventually).
        // Verify that the entry can be retrieve back.
        let sql = format!(
            "SELECT id, package_uuid, package_name, latest_instance_id
            FROM {}
            WHERE id = ?1",
            self.table_fullname
        );
        let mut stmt = conn.prepare(&sql)?;
        let row = stmt.query_row([self.id], |row| {
            let id: u64 = row.get(0)?;
            let package_uuid: String = row.get(1)?;
            let package_name: String = row.get(2)?;
            let latest_instance_id: Option<u64> = row.get(3)?;
            Ok((id, package_uuid, package_name, latest_instance_id))
        })?;
        if row.0 != self.id
            || row.1 != self.package_uuid
            || row.2 != self.package_name
            || row.3 != self.latest_instance_id
        {
            log::error!("Sanity test failed for {:?}", self);
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        Ok(())
    }

    fn update_latest_instance_id_in_db(
        &mut self,
        conn: &Connection,
        new_latest_id: u64,
    ) -> rusqlite::Result<()> {
        if self.latest_instance_id.is_none() {
            log::error!("Missing latest_instance_id for update");
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let sql = format!(
            "UPDATE {} SET latest_instance_id = ?1 WHERE id = ?2",
            self.table_fullname
        );
        let mut stmt = conn.prepare(&sql)?;
        stmt.execute([new_latest_id, self.id])?;
        self.latest_instance_id = Some(new_latest_id);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct PackageInstance {
    id: u64,
    parent_id: u64,     // Foreign key into Package table.
    package_id: String, // Package id on the network.
}

impl DBTable for PackageInstance {
    fn create_table(
        conn: &Connection,
        workdir_name: String,
        namespace: Option<String>,
        _name_suffix: Option<String>,
    ) -> rusqlite::Result<()> {
        // TODO Recreate the table if not the latest schema version.
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {0}_{1}_package_instance (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                parent_id       INTEGER NOT NULL REFERENCES {0}_{1}_package (id) ON DELETE CASCADE,
                package_id      TEXT NOT NULL
            )",
            workdir_name,
            namespace.unwrap_or_else(|| "sui".to_string())
        );
        conn.execute(&sql, [])?;
        Ok(())
    }
}

impl PackageInstance {
    fn insert_in_db(&mut self, conn: &Connection, package: &Package) -> rusqlite::Result<()> {
        let table_name = format!("{}_instance", package.table_fullname);
        let sql = format!(
            "INSERT INTO {0} (parent_id, package_id)
            VALUES ({1}, \"{2}\") RETURNING id",
            table_name, package.id, self.package_id
        );
        // log::info!("DOING SQL: {}", sql);
        let mut stmt = conn.prepare(&sql)?;
        self.id = stmt.query_row([], |row| row.get(0))?;

        // Runtime test (Remove eventually).
        // Verify that the entry can be retrieve back.
        let sql = format!(
            "SELECT id, parent_id, package_id
            FROM {}
            WHERE id = ?1",
            table_name
        );
        let mut stmt = conn.prepare(&sql)?;
        let row = stmt.query_row([self.id], |row| {
            let id: u64 = row.get(0)?;
            let parent_id: u64 = row.get(1)?;
            let package_id: String = row.get(2)?;
            Ok((id, parent_id, package_id))
        })?;
        if row.0 != self.id || row.1 != self.parent_id || row.2 != self.package_id {
            log::error!("Sanity test failed for {:?}", self);
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        Ok(())
    }

    fn insert_event_in_db(
        &self,
        conn: &Connection,
        package: &Package,
        name_suffix: String,
        event: &mut SuiEvent,
    ) -> rusqlite::Result<()> {
        // Create the SuiEvent table as needed.
        let table_name = format!("{}_event_{}", package.table_prefix, name_suffix);

        let sql = format!(
            "INSERT INTO {0} (package_instance_id, timestamp, event_json)
            VALUES ({1}, {2}, \"{3}\") RETURNING id",
            table_name, self.id, event.timestamp_ms, event.event_json
        );
        // log::info!("DOING SQL: {}", sql);
        let mut stmt = conn.prepare(&sql)?;
        event.id = stmt.query_row([], |row| row.get(0))?;
        Ok(())
    }
}

#[derive(Debug)]
struct SuiEvent {
    id: u64, // Sequence number within this table. Event assumed inserted in chronological order.
    package_instance_id: u64, // Foreign key into PackageInstance table.
    timestamp_ms: u64, // milliseconds. Also in results, but put here for sorting convenience.
    event_json: String, // This is the content of the "result" field (JSON object).
}

impl DBTable for SuiEvent {
    fn create_table(
        conn: &Connection,
        workdir_name: String,
        namespace: Option<String>,
        name_suffix: Option<String>,
    ) -> rusqlite::Result<()> {
        // TODO Recreate the table if not the latest schema version.
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {0}_{1}_event_{2} (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                package_instance_id INTEGER NOT NULL REFERENCES {0}_{1}_package_instance (id) ON DELETE CASCADE,
                timestamp       INTEGER NOT NULL,                
                event_json      TEXT NOT NULL
            )",
            workdir_name,
            namespace.unwrap_or_else(|| "sui".to_string()),
            name_suffix.unwrap_or_else(|| "default".to_string()),
        );
        conn.execute(&sql, [])?;
        Ok(())
    }
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

// Schema: global variables.
// This table have a single entry.
const SCHEMA_VERSION: &str = "0.0.1";
#[derive(Debug)]
struct DBSuibaseConfig {
    id: i32,
    version: String, // x.y.z *schema* version.
    workdir_name: String,
}

impl DBTable for DBSuibaseConfig {
    fn create_table(
        conn: &Connection,
        workdir_name: String,
        namespace: Option<String>,
        _name_suffix: Option<String>,
    ) -> rusqlite::Result<()> {
        // TODO Recreate the table if not the latest schema version.
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {0}_{1}_config (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                schema_version  TEXT NOT NULL
            )",
            workdir_name,
            namespace.unwrap_or_else(|| "sui".to_string())
        );
        conn.execute(&sql, [])?;
        Ok(())
    }
}
impl DBSuibaseConfig {
    pub fn new(workdir_name: String) -> Self {
        Self {
            id: 0,
            version: SCHEMA_VERSION.to_string(),
            workdir_name,
        }
    }
    pub fn insert(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        let sql = format!(
            "INSERT INTO suibase_globals (version, workdir_name)
            VALUES ({0}, {1})",
            self.version, self.workdir_name
        );
        let mut stmt = conn.prepare(&sql)?;
        self.id = stmt.query_row([], |row| row.get(0))?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct DBWorkerParams {
    globals: Globals,
    event_rx: Arc<Mutex<GenericRx>>,
    event_tx: GenericTx,
    workdir_idx: WorkdirIdx,
    workdir_name: String,
}

impl DBWorkerParams {
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

pub struct DBWorker {
    auto_thread: AutoThread<DBWorkerThread, DBWorkerParams>,
}

impl DBWorker {
    pub fn new(params: DBWorkerParams) -> Self {
        Self {
            auto_thread: AutoThread::new("DBWorker".to_string(), params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> anyhow::Result<()> {
        self.auto_thread.run(subsys).await
    }
}

#[derive(Debug, Default)]
struct DBManagement {
    conn: Option<Connection>,
    schema_ok: bool, // Set when all tables have been verified to exist.
}

impl DBManagement {
    pub fn new() -> Self {
        Self {
            conn: None,
            schema_ok: false,
        }
    }
}

struct DBWorkerThread {
    thread_name: String,
    params: DBWorkerParams,
    db: DBManagement,
    workdir: Workdir,
}

#[async_trait]
impl Runnable<DBWorkerParams> for DBWorkerThread {
    fn new(thread_name: String, params: DBWorkerParams) -> Self {
        Self {
            thread_name,
            params,
            db: DBManagement::new(),
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

impl DBWorkerThread {
    async fn update_globals_in_db(&mut self) {
        // Check to update globals in DB.
        //
        // If a difference is found two things happen:
        //  - The DB is updated for all changed fields.
        //  - The websocket user(s) receive a single DB change notification.
    }

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
        let workdir_name = if let Some(workdir_idx) = msg.workdir_idx {
            if workdir_idx != self.params.workdir_idx {
                log::error!(
                    "Unexpected workdir_idx {:?} (expected {:?})",
                    workdir_idx,
                    self.params.workdir_idx
                );
            }
            common::shared_types::WORKDIRS_KEYS[workdir_idx as usize]
        } else {
            log::error!("Unexpected workdir_idx {:?}", msg);
            return;
        };

        // Producer of add_sui_event should always set the Suibase uuid in msg.data_string.
        let package_uuid = if let Some(package_uuid) = msg.params(0) {
            package_uuid
        } else {
            log::error!("Missing Suibase package UUID in params(0) {:?}", msg);
            return;
        };

        let package_name = if let Some(package_name) = msg.params(1) {
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
        let package_id = package_id[2..].to_string();

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

        // TODO Handle re-establish a DB connection if disconnected.
        //      For now, just return with an error if no DB connection.
        if self.db.conn.is_none() {
            log::error!("No DB connection to handle Sui event {:?}", data_json);
            return;
        }
        let conn = self.db.conn.as_ref().unwrap();

        // Get the related package fields from the DB.
        // Create the Package/PackageInstance in DB as needed.
        let objs = Package::get_objs_from_db(
            conn,
            workdir_name.to_string(),
            None,
            package_uuid,
            package_name,
            package_id,
        );
        let (package, package_instance): (Package, PackageInstance) = match objs {
            Some(boxed_tuple) => *boxed_tuple,
            None => {
                log::error!("Failed to get Package from DB {:?}", data_json);
                return;
            }
        };
        // TODO: If new latest, purge very old events in all tables.

        // Insert event into proper table (should be already created).
        let name_suffix = format!("{}_{}", sub_table_name, event_level);
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
        let event_json = event_json.replace('\"', "\"\"");
        let mut new_sui_event = SuiEvent::new(package_instance.id, timestamp_ms, event_json);
        if let Err(e) =
            package_instance.insert_event_in_db(conn, &package, name_suffix, &mut new_sui_event)
        {
            log::error!("Failed to insert SuiEvent in DB {:?}", e);
        }

        // TODO Broadcast the sequence number increment of this sui_event object to websocket users.
    }

    async fn open_db(&mut self) -> bool {
        // Get copy of latest workdir info from globals.
        let workdir =
            self.params.globals.get_workdir_by_idx(self.params.workdir_idx)
                .await;
        if workdir.is_none() {
            log::error!(
                "Failed to get workdir info for workdir_idx {:?}",
                self.params.workdir_idx
            );
            return false;
        }
        self.workdir = workdir.unwrap();

        // Open a DB connection to the sqlite.db file. Will create it if does not exists.
        let path = self.workdir.path();
        let path = path.join("indexer");
        if std::fs::create_dir_all(&path).is_err() {
            log::error!("Failed to create indexer directory: {:?}", path);
            return false;
        }
        let _pathname = path.join("sqlite.db");
        //let conn = Connection::open(pathname);
        // TODO For now, develop the concept in-memory and serve data through JSON-RPC.
        //      Later, if the "load" gets too high, then move to a file-based DB.
        let conn = Connection::open_in_memory();
        if conn.is_err() {
            log::error!("Failed to open sqlite database: {:?}", conn);
            return false;
        }
        let conn = conn.unwrap();

        if let Err(e) = conn.execute("PRAGMA foreign_keys = ON;", []) {
            log::error!("Failed to enable foreign keys {:?}", e);
            return false;
        }

        // Create some tables in the schema to simplify access from this code later.
        // This is a single row table with frequently used globals.
        if let Err(e) = DBSuibaseConfig::create_table(&conn, "all".to_string(), None, None) {
            log::error!("Failed to create suibase_globals table {:?}", e);
            return false;
        }

        // Create tables that exists for each workdir.
        for workdir_name in common::shared_types::WORKDIRS_KEYS.iter() {
            if let Err(e) =
                DBSuibaseConfig::create_table(&conn, workdir_name.to_string(), None, None)
            {
                log::error!(
                    "Failed to create {} SuibaseConfig table {:?}",
                    workdir_name,
                    e
                );
                return false;
            }

            if let Err(e) = Package::create_table(&conn, workdir_name.to_string(), None, None) {
                log::error!("Failed to create {} package table {:?}", workdir_name, e);
                return false;
            }

            if let Err(e) =
                PackageInstance::create_table(&conn, workdir_name.to_string(), None, None)
            {
                log::error!(
                    "Failed to create {} package_instance table {:?}",
                    workdir_name,
                    e
                );
                return false;
            }

            // Create the console SuiEvent tables (one table per level).
            for level in basic_types::EVENT_LEVEL_MIN..=basic_types::EVENT_LEVEL_MAX {
                let name_suffix = format!("console_{}", level);
                if let Err(e) =
                    SuiEvent::create_table(&conn, workdir_name.to_string(), None, Some(name_suffix))
                {
                    log::error!(
                        "Failed to create {} console level={} event table {:?}",
                        workdir_name,
                        level,
                        e
                    );
                    return false;
                }
            }

            // Create the user SuiEvent table.
            let name_suffix = "user_0";
            if let Err(e) = SuiEvent::create_table(
                &conn,
                workdir_name.to_string(),
                None,
                Some(name_suffix.to_string()),
            ) {
                log::error!("Failed to create {} user event table {:?}", workdir_name, e);
                return false;
            }
        }

        // All success. This is a good DB connection.
        log::info!("Open connection success");
        self.db.conn = Some(conn);
        true
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        // Take mutable ownership of the event_rx channel as long this thread is running.
        let event_rx = Arc::clone(&self.params.event_rx);
        let mut event_rx = event_rx.lock().await;

        // Open the database connection.
        if self.db.conn.is_none() && !self.open_db().await {
            // Delay of 5 seconds before retrying.
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            return;
        }

        while !subsys.is_shutdown_requested() {
            /*let ws_stream_future =
            futures::FutureExt::fuse(self.websocket.read.as_mut().unwrap().next());*/
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
