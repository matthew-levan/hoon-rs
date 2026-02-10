use serde_json::Value;

#[derive(Debug, Clone)]
pub struct HoonLspConfig {
    pub debounce_ms: u64,
    pub workspace_refresh_ms: u64,
    pub max_workspace_files: usize,
    pub follow_symlinks: bool,
}

impl Default for HoonLspConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 75,
            workspace_refresh_ms: 30_000,
            max_workspace_files: 10_000,
            follow_symlinks: true,
        }
    }
}

impl HoonLspConfig {
    pub fn merge_json(&mut self, value: &Value) {
        let obj = value
            .get("hoonLsp")
            .and_then(Value::as_object)
            .or_else(|| value.as_object());
        let Some(obj) = obj else {
            return;
        };

        if let Some(ms) = obj.get("debounceMs").and_then(Value::as_u64) {
            self.debounce_ms = ms;
        }
        if let Some(ms) = obj.get("workspaceRefreshMs").and_then(Value::as_u64) {
            self.workspace_refresh_ms = ms;
        }
        if let Some(max) = obj.get("maxWorkspaceFiles").and_then(Value::as_u64) {
            self.max_workspace_files = max as usize;
        }
        if let Some(follow) = obj.get("followSymlinks").and_then(Value::as_bool) {
            self.follow_symlinks = follow;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::HoonLspConfig;

    #[test]
    fn merge_json_reads_top_level_keys() {
        let mut cfg = HoonLspConfig::default();
        cfg.merge_json(&json!({
            "debounceMs": 250,
            "workspaceRefreshMs": 15000,
            "maxWorkspaceFiles": 77,
            "followSymlinks": false
        }));

        assert_eq!(cfg.debounce_ms, 250);
        assert_eq!(cfg.workspace_refresh_ms, 15_000);
        assert_eq!(cfg.max_workspace_files, 77);
        assert!(!cfg.follow_symlinks);
    }

    #[test]
    fn merge_json_reads_nested_hoonlsp_keys() {
        let mut cfg = HoonLspConfig::default();
        cfg.merge_json(&json!({
            "hoonLsp": {
                "debounceMs": 10,
                "maxWorkspaceFiles": 123
            }
        }));

        assert_eq!(cfg.debounce_ms, 10);
        assert_eq!(cfg.max_workspace_files, 123);
    }
}
