use home::home_dir;
use std::path::Path;

pub(crate) struct SuibaseRoot {
    // Parent to all internal variables, except for the per workdir ones (see suibase_workdir.rs)
    is_installed: bool, // false on OS access failure.

    // Absolute path to suibase installation.
    // (e.g. /home/johndoe/suibase )
    suibase_path: String,

    // Absolute path to suibase/workdirs
    // (e.g. /home/johndoe/suibase/workdirs )
    workdirs_path: String,
}

impl SuibaseRoot {
    pub fn new() -> SuibaseRoot {
        // Create with default init state + refresh_state().
        let mut new_obj = SuibaseRoot {
            is_installed: false,
            suibase_path: String::new(),
            workdirs_path: String::new(),
        };
        new_obj.refresh_state();
        new_obj
    }

    pub fn is_installed(self: &mut SuibaseRoot) -> bool {
        self.refresh_state();
        self.is_installed
    }

    #[allow(dead_code)]
    pub fn suibase_path(self: &SuibaseRoot) -> &str {
        &self.suibase_path
    }

    pub fn workdirs_path(self: &SuibaseRoot) -> &str {
        &self.workdirs_path
    }

    pub fn refresh_state(self: &mut SuibaseRoot) {
        if let Some(mut path_buf) = home_dir() {
            path_buf.push("suibase");
            self.suibase_path = path_buf.to_string_lossy().to_string();

            path_buf.push("workdirs");
            self.workdirs_path = path_buf.to_string_lossy().to_string();
        }

        let base_path_ok = if self.suibase_path.is_empty() {
            false
        } else {
            Path::new(&self.suibase_path).exists()
        };

        let workdirs_path_ok = if self.workdirs_path.is_empty() {
            false
        } else {
            Path::new(&self.workdirs_path).exists()
        };

        self.is_installed = base_path_ok && workdirs_path_ok;
    }
}

#[cfg(test)]
mod tests {
    use super::SuibaseRoot;

    #[test]
    fn test_new() {
        let mut sb = SuibaseRoot::new();
        assert_eq!(sb.is_installed(), true);
        let path = sb.suibase_path();
        assert_eq!(path.is_empty(), false);
        assert_eq!(path.ends_with("suibase"), true);
        let workdir_path = sb.workdirs_path();
        assert_eq!(workdir_path.is_empty(), false);
        assert_eq!(workdir_path.ends_with("suibase/workdirs"), true);
    }

    #[test]
    fn test_refresh_state() {
        let mut sb = SuibaseRoot::new();
        sb.refresh_state();
        assert_eq!(sb.is_installed(), true);
        let path = sb.suibase_path();
        assert_eq!(path.is_empty(), false);
        assert_eq!(path.ends_with("suibase"), true);
        let workdir_path = sb.workdirs_path();
        assert_eq!(workdir_path.is_empty(), false);
        assert_eq!(workdir_path.ends_with("suibase/workdirs"), true);
    }
}
