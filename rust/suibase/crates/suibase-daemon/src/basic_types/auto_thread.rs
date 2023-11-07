// A Tokio thread integrated with tokio-graceful-shutdown crates.
//
// For the parent the new child thread is "start and forget".
//
// On panic, will log a message and auto-restart.
//
// Will cleanly self-exit on SIGTERM, Ctrl-C etc...
//

use anyhow::Result;
use axum::async_trait;
use std::marker::PhantomData;
use tokio::time::Duration;
use tokio_graceful_shutdown::{ErrorAction, SubsystemBuilder, SubsystemHandle};

#[async_trait]
pub trait Runnable<Parameter: Send> {
    fn new(name: String, params: Parameter) -> Self;
    async fn run(self, subsys: SubsystemHandle) -> Result<()>;
}

pub struct AutoThread<Thread: Runnable<Parameter>, Parameter: Send> {
    pub name: String,
    pub params: Parameter,
    _thread: PhantomData<Thread>,
}

impl<Thread: Runnable<Parameter>, Parameter: Send> AutoThread<Thread, Parameter> {
    pub fn new(name: String, params: Parameter) -> Self {
        Self {
            name,
            params,
            _thread: PhantomData,
        }
    }
}

#[async_trait]
impl<Thread: Runnable<Parameter> + Send + 'static, Parameter: Send + Clone> Runnable<Parameter>
    for AutoThread<Thread, Parameter>
{
    fn new(name: String, params: Parameter) -> Self {
        Self {
            name,
            params,
            _thread: PhantomData,
        }
    }

    async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        let outer_thread_name = format!("{}-outer", self.name);
        let inner_thread_name = format!("{}-inner", self.name);
        log::info!("{} started", outer_thread_name);
        loop {
            // Create an instance of the Thread. If it panics, then
            // we will just start a new instance on next loop iteration.
            let inner_thread = Thread::new(self.name.clone(), self.params.clone());

            let nested_subsys = subsys.start(
                SubsystemBuilder::new(inner_thread_name.clone(), |a| inner_thread.run(a))
                    .on_failure(ErrorAction::CatchAndLocalShutdown)
                    .on_panic(ErrorAction::CatchAndLocalShutdown),
            );

            if let Err(err) = nested_subsys.join().await {
                // TODO Restart the process on excess of errors for tentative recovery (e.g. memory leaks?)
                log::error!("{}: {}", inner_thread_name, err);
                // Something went wrong, wait a couple of second before restarting
                // the inner server, but do not block from exiting.
                for _ in 0..4 {
                    if subsys.is_shutdown_requested() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }

            if subsys.is_shutdown_requested() {
                break;
            }

            log::info!("{} restarting...", inner_thread_name);
        }
        log::info!("{} shutting down - normal exit", outer_thread_name);
        Ok(())
    }
}
