use chrono::{DateTime, Utc};
use log::warn;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeHistoryItem {
    pub issue_id: String,
    pub issue_key: String,
    pub history_id: String,
    pub author_account_id: Option<String>,
    pub author_display_name: Option<String>,
    pub field: String,
    pub field_type: Option<String>,
    pub from_value: Option<String>,
    pub from_string: Option<String>,
    pub to_value: Option<String>,
    pub to_string: Option<String>,
    pub changed_at: DateTime<Utc>,
}

impl ChangeHistoryItem {
    pub fn extract_from_raw_json(issue_id: &str, issue_key: &str, raw_json: &str) -> Vec<Self> {
        let mut items = Vec::new();

        let json: Value = match serde_json::from_str(raw_json) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to parse raw_json for {}: {}", issue_key, e);
                return items;
            }
        };

        let changelog = json.get("changelog");
        if changelog.is_none() {
            log::debug!(
                "No changelog found in raw_json for {} (this is normal for issues with no changes)",
                issue_key
            );
            return items;
        }

        let histories = match changelog.and_then(|c| c.get("histories")) {
            Some(Value::Array(arr)) => arr,
            _ => {
                log::debug!("No histories array found in changelog for {}", issue_key);
                return items;
            }
        };

        for history in histories {
            let history_id = history
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let author_account_id = history
                .get("author")
                .and_then(|a| a.get("accountId"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let author_display_name = history
                .get("author")
                .and_then(|a| a.get("displayName"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let changed_at = history
                .get("created")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let change_items = match history.get("items") {
                Some(Value::Array(arr)) => arr,
                _ => continue,
            };

            for item in change_items {
                let field = match item.get("field").and_then(|v| v.as_str()) {
                    Some(f) => f.to_string(),
                    None => continue,
                };

                let field_type = item
                    .get("fieldtype")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let from_value = item
                    .get("from")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let from_string = item
                    .get("fromString")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let to_value = item
                    .get("to")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let to_string = item
                    .get("toString")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                items.push(ChangeHistoryItem {
                    issue_id: issue_id.to_string(),
                    issue_key: issue_key.to_string(),
                    history_id: history_id.clone(),
                    author_account_id: author_account_id.clone(),
                    author_display_name: author_display_name.clone(),
                    field,
                    field_type,
                    from_value,
                    from_string,
                    to_value,
                    to_string,
                    changed_at,
                });
            }
        }

        items
    }
}
