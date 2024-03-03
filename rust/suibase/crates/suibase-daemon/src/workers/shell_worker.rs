use std::process::Command;

use crate::shared_types::Globals;

use common::basic_types::{GenericChannelMsg, GenericRx, WorkdirIdx};

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

pub struct ShellWorker {
    _globals: Globals,
    event_rx: GenericRx,
    workdir_idx: Option<WorkdirIdx>,
}

impl ShellWorker {
    pub fn new(globals: Globals, event_rx: GenericRx, workdir_idx: Option<WorkdirIdx>) -> Self {
        Self {
            _globals: globals,
            event_rx,
            workdir_idx,
        }
    }

    async fn do_exec(&mut self, msg: GenericChannelMsg) {
        // No error return here. Once the execution is completed, the output
        // of the response is returned to requester with a one shot message.
        //
        // If the response starts with "Error:", then an error was detected.
        //
        // Some effects are also possible on globals, particularly
        // for sharing large results.
        //
        log::info!(
            "do_exec() msg {:?} for workdir_idx={:?}",
            msg,
            msg.workdir_idx
        );

        let resp = if msg.workdir_idx != self.workdir_idx {
            log::error!(
                "do_exec() msg {:?} for workdir_idx={:?} does not match self.workdir_idx={:?}",
                msg,
                msg.workdir_idx,
                self.workdir_idx
            );
            format!(
                "Error: unexpected workdir_idx {:?} != {:?}",
                msg.workdir_idx, self.workdir_idx
            )
        } else if msg.event_id != common::basic_types::EVENT_EXEC {
            log::error!("Unexpected event_id {:?}", msg.event_id);
            format!("Error: Unexpected event_id {:?}", msg.event_id)
        } else if let Some(cmd) = &msg.command {
            // Execute the command as if it was a bash script.
            let output = Command::new("bash").arg("-c").arg(cmd).output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let resp = format!("{}{}", stdout, stderr);
                    if output.status.success() && stderr.is_empty() {
                        resp
                    } else {
                        format!("Error: {}", resp)
                    }
                }
                Err(e) => {
                    let error_msg = format!(
                        "Error: do_exec({:?}, {:?}) error 1: {}",
                        msg.workdir_idx, cmd, e
                    );
                    log::error!("{}", error_msg);
                    error_msg
                }
            }
        } else {
            let error_msg = format!(
                "Error: do_exec({:?}, None) error 2: No command to execute",
                msg.workdir_idx
            );
            log::error!("{}", error_msg);
            error_msg
        };

        if let Some(resp_channel) = msg.resp_channel {
            if let Err(e) = &resp_channel.send(resp) {
                let error_msg = format!(
                    "Error: do_exec({:?}, {:?}) error 3: {}",
                    msg.workdir_idx, msg.command, e
                );
                log::error!("{}", error_msg);
            }
        }
    }

    async fn event_loop(&mut self, subsys: &SubsystemHandle) {
        while !subsys.is_shutdown_requested() {
            // Wait for a message.
            if let Some(msg) = self.event_rx.recv().await {
                // Process the message.
                self.do_exec(msg).await;
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started");

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
