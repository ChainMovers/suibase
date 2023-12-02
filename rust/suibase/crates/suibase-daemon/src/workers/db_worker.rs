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

use crate::{
    basic_types::{
        self, AutoThread, DBTable, GenericChannelMsg, GenericRx, GenericTx, Runnable, WorkdirIdx,
    },
    shared_types::{Globals, GlobalsWorkdirsST, Workdir},
};

use rusqlite::Connection;

use axum::async_trait;

use tokio::sync::Mutex;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

// Schema: One entry per Package.
#[derive(Debug)]
struct Package {
    id: i32,
    package_uuid: String,
    package_name: String,
    latest_instance_id: Option<i32>,
}

#[derive(Debug)]
struct PackageInstance {
    id: i32,
    package_id: String,
}

#[derive(Debug)]
struct SuiEvent {
    id: i32,
    package_id: i32,
    event_json: String,
}

impl DBTable for Package {
    fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        // TODO Recreate the table if not the latest schema version.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS package (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                schema_version  TEXT NOT NULL,
                package_uuid    TEXT NOT NULL,
                package_name    TEXT NOT NULL,
                latest_instance_id INTEGER
            )",
            [],
        )?;
        Ok(())
    }
}

impl Package {
    pub fn new(package_uuid: String, package_name: String) -> Self {
        Self {
            id: 0,
            package_uuid,
            package_name,
            latest_instance_id: None,
        }
    }
    pub fn insert(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        let mut stmt = conn.prepare(
            "INSERT INTO package (package_uuid, package_name, latest_instance_id)
            VALUES (?1, ?2, ?3)",
        )?;
        let latest_instance_id = self.latest_instance_id.unwrap_or(0);
        let mut rows = stmt.query([
            self.package_uuid.clone(),
            self.package_name.clone(),
            latest_instance_id.to_string(),
        ])?;
        self.id = rows.next()?.unwrap().get(0)?;
        Ok(())
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
    fn create_table(conn: &Connection) -> rusqlite::Result<()> {
        // TODO Recreate the table if not the latest schema version.
        conn.execute(
            "CREATE TABLE IF NOT EXISTS suibase_globals (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                schema_version  TEXT NOT NULL,
                workdir_name    TEXT NOT NULL
            )",
            [],
        )?;
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
        let mut stmt = conn.prepare(
            "INSERT INTO suibase_globals (version, workdir_name)
            VALUES (?1, ?2)",
        )?;
        let mut rows = stmt.query([self.version.clone(), self.workdir_name.clone()])?;
        self.id = rows.next()?.unwrap().get(0)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct DBWorkerParams {
    globals: Globals,
    event_rx: Arc<Mutex<GenericRx>>,
    event_tx: GenericTx,
    workdir_idx: WorkdirIdx,
}

impl DBWorkerParams {
    pub fn new(
        globals: Globals,
        event_rx: GenericRx,
        event_tx: GenericTx,
        workdir_idx: WorkdirIdx,
    ) -> Self {
        Self {
            globals,
            event_rx: Arc::new(Mutex::new(event_rx)),
            event_tx,
            workdir_idx,
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
        log::info!("started");

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("shutting down - normal exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("shutting down - normal exit (1)");
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

    async fn open_db(&mut self) -> bool {
        // Get copy of latest workdir info from globals.
        let workdir =
            GlobalsWorkdirsST::get_workdir_by_idx(&self.params.globals, self.params.workdir_idx)
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
        // Create now all the tables in the schema to simplify access from this code later.
        if let Err(e) = DBSuibaseConfig::create_table(&conn) {
            log::error!("Failed to create suibase_globals table {:?}", e);
            return false;
        }
        if let Err(e) = Package::create_table(&conn) {
            log::error!("Failed to create package table {:?}", e);
            return false;
        }

        // All success. This is a good DB connection.
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
