use home::home_dir;
use std::path::Path;

pub(crate) struct SuiBaseRoot {
    // Parent to all internal variables, except for the per workdir ones (see sui_base_workdir.rs)
    is_installed: bool, // false on OS access failure.

    // Absolute path to suibase installation.
    // (e.g. /home/johndoe/suibase )
    sui_base_path: String,

    // Absolute path to suibase/workdirs
    // (e.g. /home/johndoe/suibase/workdirs )
    workdirs_path: String,
}

impl SuiBaseRoot {
    pub fn new() -> SuiBaseRoot {
        // Create with default init state + refresh_state().
        let mut new_obj = SuiBaseRoot {
            is_installed: false,
            sui_base_path: String::new(),
            workdirs_path: String::new(),
        };
        new_obj.refresh_state();
        new_obj
    }

    pub fn is_installed(self: &mut SuiBaseRoot) -> bool {
        self.refresh_state();
        self.is_installed
    }

    #[allow(dead_code)]
    pub fn sui_base_path(self: &SuiBaseRoot) -> &str {
        &self.sui_base_path
    }

    pub fn workdirs_path(self: &SuiBaseRoot) -> &str {
        &self.workdirs_path
    }

    pub fn refresh_state(self: &mut SuiBaseRoot) {
        if let Some(mut path_buf) = home_dir() {
            path_buf.push("suibase");
            self.sui_base_path = path_buf.to_string_lossy().to_string();

            path_buf.push("workdirs");
            self.workdirs_path = path_buf.to_string_lossy().to_string();
        }

        let base_path_ok = if self.sui_base_path.is_empty() {
            false
        } else {
            Path::new(&self.sui_base_path).exists()
        };

        let workdirs_path_ok = if self.workdirs_path.is_empty() {
            false
        } else {
            Path::new(&self.workdirs_path).exists()
        };

        self.is_installed = base_path_ok && workdirs_path_ok;
    }
}
