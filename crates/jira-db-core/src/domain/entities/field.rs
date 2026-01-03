use serde::{Deserialize, Serialize};

/// Represents a JIRA field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraField {
    /// Field ID (e.g., "summary", "customfield_10020")
    pub id: String,
    /// Field key (often same as id)
    pub key: String,
    /// Human-readable field name
    pub name: String,
    /// Whether this is a custom field
    pub custom: bool,
    /// Whether this field can be searched
    pub searchable: bool,
    /// Whether this field is navigable
    pub navigable: bool,
    /// Whether this field is orderable
    pub orderable: bool,
    /// Schema information for the field
    pub schema_type: Option<String>,
    /// Schema items (for array types)
    pub schema_items: Option<String>,
    /// Schema system (e.g., "user", "priority")
    pub schema_system: Option<String>,
    /// Custom field type (for custom fields)
    pub schema_custom: Option<String>,
    /// Custom field ID (numeric part)
    pub schema_custom_id: Option<i64>,
}

impl JiraField {
    /// Check if this field should be expanded into a column
    /// Returns true for fields that are useful in a flattened table
    pub fn is_expandable(&self) -> bool {
        // Skip internal JIRA fields that are not useful
        let skip_fields = [
            "statuscategorychangedate",
            "workratio",
            "lastViewed",
            "watches",
            "thumbnail",
            "votes",
            "worklog",
            "comment",
            "attachment",
            "subtasks",
            "issuelinks",
            "timetracking",
            "timeoriginalestimate",
            "timespent",
            "timeestimate",
            "aggregatetimeoriginalestimate",
            "aggregatetimespent",
            "aggregatetimeestimate",
            "aggregateprogress",
            "progress",
        ];

        !skip_fields.contains(&self.id.as_str())
    }

    /// Get the DuckDB column type for this field
    pub fn get_column_type(&self) -> &'static str {
        match self.schema_type.as_deref() {
            Some("string") => "TEXT",
            Some("number") => "DOUBLE",
            Some("datetime") => "TIMESTAMP",
            Some("date") => "DATE",
            Some("array") => "JSON",
            Some("user") => "TEXT",       // Store displayName
            Some("priority") => "TEXT",   // Store name
            Some("status") => "TEXT",     // Store name
            Some("issuetype") => "TEXT",  // Store name
            Some("resolution") => "TEXT", // Store name
            Some("project") => "TEXT",    // Store key
            Some("option") => "TEXT",     // Store value
            Some("option-with-child") => "TEXT",
            Some("issuelink") => "JSON",
            _ => "TEXT", // Default to TEXT for unknown types
        }
    }

    /// Get the safe column name for this field (for SQL)
    pub fn get_safe_column_name(&self) -> String {
        // Replace special characters with underscore
        let name = self
            .id
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .to_lowercase();

        // Prefix with 'cf_' for custom fields if not already prefixed
        if self.custom && !name.starts_with("customfield_") {
            format!("cf_{}", name)
        } else {
            name
        }
    }
}
