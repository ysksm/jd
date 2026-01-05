/// Represents a JIRA issue transition
#[derive(Debug, Clone)]
pub struct TransitionDto {
    /// Transition ID
    pub id: String,
    /// Transition name (e.g., "Start Progress", "Done")
    pub name: String,
    /// Target status after transition
    pub to_status: String,
    /// Target status category (e.g., "new", "indeterminate", "done")
    pub to_status_category: Option<String>,
}

impl TransitionDto {
    pub fn new(
        id: String,
        name: String,
        to_status: String,
        to_status_category: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            to_status,
            to_status_category,
        }
    }
}
