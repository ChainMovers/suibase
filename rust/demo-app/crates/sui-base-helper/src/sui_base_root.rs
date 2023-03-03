use home::home_dir;
use std::path::Path;

pub(crate) struct SuiBaseRoot {
    is_sui_base_installed: bool, // false on OS access failure.

    // Absolute path to sui-base installation.
    // (e.g. /home/johndoe/sui-base )
    sui_base_path: String,

    // Absolute path to sui-base/workdirs
    // (e.g. /home/johndoe/sui-base/workdirs )
    workdirs_path: String,
}

impl SuiBaseRoot {
    pub fn new() -> SuiBaseRoot {
        // Create with default init state + refresh_state().
        let mut new_obj = SuiBaseRoot {
            is_sui_base_installed: false,
            sui_base_path: String::new(),
            workdirs_path: String::new(),
        };
        new_obj.refresh_state();
        new_obj
    }

    pub fn is_sui_base_installed(self: &mut SuiBaseRoot) -> bool {
        self.refresh_state();
        self.is_sui_base_installed
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
            path_buf.push("sui-base");
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

        self.is_sui_base_installed = base_path_ok && workdirs_path_ok;
    }
}
