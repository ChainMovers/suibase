// Thread to run shell commands, particularly Suibase scripts.
//
// One ShellWorker should be instantiated for each workdir. This will allow commands to:
//   - Run concurrently when for different workdir.
//   - Run sequentially when for the same workdir.
//
use std::{path::PathBuf, process::Command};

use anyhow::Result;
use tokio_graceful_shutdown::{FutureExt, SubsystemHandle};

use crate::basic_types::{GenericChannelMsg, GenericRx, WorkdirIdx};

use home::home_dir;

pub struct ShellWorker {
    event_rx: GenericRx,
    workdir_idx: Option<WorkdirIdx>,
    home_dir: PathBuf,
}

impl ShellWorker {
    pub fn new(event_rx: GenericRx, workdir_idx: Option<WorkdirIdx>) -> Self {
        let home_dir = if let Some(home_dir) = home_dir() {
            home_dir
        } else {
            PathBuf::from("/tmp")
        };
        Self {
            event_rx,
            workdir_idx,
            home_dir,
        }
    }

    fn do_exec(&mut self, msg: GenericChannelMsg) {
        // No error return here. Once the execution is completed, the output
        // of the response is returned to requester with a one shot message.
        //
        // If the response starts with "Error:", then an error was detected.
        let mut pre_call_error: Option<String> = None;
        let resp: Option<String>;

        let log_details = if let Some(command) = &msg.command {
            !command.ends_with("status")
        } else {
            false
        };

        if msg.workdir_idx != self.workdir_idx {
            pre_call_error = Some(format!(
                "Error: unexpected workdir_idx {:?} != {:?}",
                msg.workdir_idx, self.workdir_idx
            ));
        } else if msg.event_id != crate::basic_types::EVENT_EXEC {
            pre_call_error = Some(format!("Error: Unexpected event_id {:?}", msg.event_id));
        } else if msg.command.is_none() {
            pre_call_error = Some(format!(
                "Error: do_exec({:?}, None): No command to execute",
                msg.workdir_idx
            ));
        };
        if let Some(pre_call_error) = pre_call_error {
            // There is an error, do not try to perform the command.
            log::error!("{}", pre_call_error);
            resp = Some(pre_call_error);
        } else {
            let cmd = &msg.command.clone().unwrap();
            let cwd = format!("{}/suibase", self.home_dir.display());

            if log_details {
                log::info!(
                    "do_exec() cwd={} cmd={:?} for workdir_idx={:?}",
                    cwd,
                    msg,
                    msg.workdir_idx
                );
            }

            // Execute the command as if it was a bash script.
            let child = Command::new("bash")
                .current_dir(cwd)
                .arg("-c")
                .arg(cmd)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn();

            if let Err(e) = &child {
                let error_msg = format!(
                    "Error: failed to spawn do_exec({:?}, {:?}) error: {}",
                    msg.workdir_idx, cmd, e
                );
                log::error!("{}", error_msg);
                resp = Some(error_msg);
            } else {
                let child = child.unwrap();
                let output = child.wait_with_output();

                match output {
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let mut outputs = if stderr.is_empty() {
                            stdout
                        } else {
                            format!("{}\n{}", stderr, stdout)
                        };
                        outputs = outputs.trim().to_string();
                        if output.status.success() {
                            resp = Some(outputs);
                        } else {
                            let error_msg = format!(
                                "Error: do_exec({:?}, {:?}) returned {}",
                                msg.workdir_idx, cmd, outputs
                            );
                            log::error!("{}", error_msg);
                            resp = Some(error_msg);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "Error: do_exec({:?}, {:?}) error 1: {}",
                            msg.workdir_idx, cmd, e
                        );
                        log::error!("{}", error_msg);
                        resp = Some(error_msg);
                    }
                }
            }
        }

        if let Some(resp_channel) = msg.resp_channel {
            let resp = if let Some(resp) = resp {
                resp
            } else {
                format!(
                    "Error: do_exec({:?}, {:?}) unexpected empty response",
                    msg.workdir_idx, msg.command
                )
            };

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
                self.do_exec(msg);
            } else {
                // Channel closed or shutdown requested.
                return;
            }
        }
    }

    pub async fn run(mut self, subsys: SubsystemHandle) -> Result<()> {
        log::info!("started for workdir index {:?}", self.workdir_idx);

        match self.event_loop(&subsys).cancel_on_shutdown(&subsys).await {
            Ok(()) => {
                log::info!("normal thread exit (2) for {:?}", self.workdir_idx);
                Ok(())
            }
            Err(_cancelled_by_shutdown) => {
                log::info!("normal thread exit (1) for {:?}", self.workdir_idx);
                Ok(())
            }
        }
    }
}
