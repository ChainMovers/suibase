// Run a webserver to serve all files in a specified directory.
//
// The tokio task is auto-restart in case of panic.

use crate::shared_types::Globals;

use anyhow::Result;
use axum::async_trait;

use common::basic_types::{AdminControllerTx, AutoThread, Runnable};

use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

#[derive(Clone)]
pub struct WebserverParams {
    globals: Globals,
    admctrl_tx: AdminControllerTx,
    website_name: String,
}

impl WebserverParams {
    pub fn new(globals: Globals, admctrl_tx: AdminControllerTx, website_name: &str) -> Self {
        Self {
            globals,
            admctrl_tx,
            website_name: website_name.to_owned(),
        }
    }
}

pub struct WebserverWorker {
    auto_thread: AutoThread<WebserverTask, WebserverParams>,
}

impl WebserverWorker {
    pub fn new(params: WebserverParams) -> Self {
        let name = format!("Webserver({})", params.website_name);
        Self {
            auto_thread: AutoThread::new(name, params),
        }
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        self.auto_thread.run(subsys).await
    }
}

struct WebserverTask {
    task_name: String,
    params: WebserverParams,
    websites_root: String,
}

#[async_trait]
impl Runnable<WebserverParams> for WebserverTask {
    fn new(task_name: String, params: WebserverParams) -> Self {
        Self {
            task_name,
            params,
            websites_root: "".to_string(),
        }
    }

    async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        // The websites (static files) are stored under "~/suibase/typescript".
        // websites_root has the ~ resolved (e.g. "/home/user_name/suibase/typescript")
        self.websites_root = format!("{}/typescript", {
            let workdirs_guard = self.params.globals.workdirs.read().await;
            let workdirs = &*workdirs_guard;
            workdirs.suibase_home().to_string()
        });

        let output = format!("started {}", self.task_name);
        log::info!("{}", output);

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(_) => {
                log::info!("normal task exit (2)");
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("{} normal task exit (1)", self.task_name);
                Ok(())
            }
        }
    }
}

impl WebserverTask {
    async fn event_loop(&mut self, _subsys: &SubsystemHandle) -> Result<()> {
        // Serve files located under self.websites_root ("~/suibase/typescript" with ~ resolved).
        let static_files_path = if self.params.website_name == "sui-explorer" {
            // Serve the explorer app
            format!("{}/sui-explorer/apps/explorer/build", self.websites_root)
        } else {
            // Serve whatever is at "~/suibase/typescript/website_name"
            format!("{}/{}", self.websites_root, self.params.website_name)
        };

        // Use tower to handle the serving of the static files + index.html
        let tower_srvc = tower_http::services::ServeDir::new(static_files_path)
            .append_index_html_on_directories(true);

        // Setup the router to always use the tower service.
        //
        // There is no caching, the files are purposely read and served on each request.
        //
        // Why? The Suibase webserver favor KISS over performance (modifying the files
        // update the "website" on next request/refresh).
        let app = axum::Router::new().fallback(
            axum::routing::get_service(tower_srvc).handle_error(|error| async move {
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            }),
        );

        // Define the address to serve on
        //
        // TODO Get this from the yaml configuration.
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 44380));
        log::info!("{} listening on {}", self.task_name, addr);

        // Run the server
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

        Ok(())
    }
}
