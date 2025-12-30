#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub project_key: String,
    pub issues_synced: usize,
    pub history_items_synced: usize,
    pub success: bool,
    pub error_message: Option<String>,
}

impl SyncResult {
    pub fn success(project_key: String, issues_synced: usize, history_items_synced: usize) -> Self {
        Self {
            project_key,
            issues_synced,
            history_items_synced,
            success: true,
            error_message: None,
        }
    }

    pub fn failure(project_key: String, error_message: String) -> Self {
        Self {
            project_key,
            issues_synced: 0,
            history_items_synced: 0,
            success: false,
            error_message: Some(error_message),
        }
    }
}
