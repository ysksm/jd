//! Structured logging for sync operations

use chrono::{DateTime, Utc};
use log::info;
use std::collections::HashMap;
use std::time::Instant;

/// Sync operation logger for structured, readable output
pub struct SyncLogger {
    project_key: String,
    start_time: Instant,
    total_steps: usize,
    current_step: usize,
}

impl SyncLogger {
    pub fn new(project_key: &str, total_steps: usize) -> Self {
        Self {
            project_key: project_key.to_string(),
            start_time: Instant::now(),
            total_steps,
            current_step: 0,
        }
    }

    pub fn start(&self) {
        info!("");
        info!("═══════════════════════════════════════════════════════════════");
        info!("[SYNC] Starting sync for project: {}", self.project_key);
        info!("═══════════════════════════════════════════════════════════════");
        info!("");
    }

    pub fn step(&mut self, name: &str) -> StepLogger {
        self.current_step += 1;
        info!("[Step {}/{}] {}", self.current_step, self.total_steps, name);
        StepLogger::new()
    }

    pub fn summary(&self, report: &SyncSummaryReport) {
        info!("");
        info!("===============================================================");
        info!("[SYNC] Summary for project: {}", self.project_key);
        info!("===============================================================");
        info!("");

        // Issue count comparison
        info!("  Issue Counts:");
        info!("    JIRA Cloud:     {}", report.jira_total_count);
        info!("    Local Database: {}", report.local_total_count);

        let count_match = report.jira_total_count == report.local_total_count;
        if count_match {
            info!("    Status: OK (counts match)");
        } else {
            let diff = report.local_total_count as i64 - report.jira_total_count as i64;
            if diff > 0 {
                info!("    Status: MISMATCH (local has {} more)", diff);
            } else {
                info!("    Status: MISMATCH (JIRA has {} more)", -diff);
            }
        }
        info!("");

        // Local database details
        info!("  Local Database Details:");
        info!("    Change history records: {}", report.local_history_count);
        info!("    Snapshots: {}", report.local_snapshot_count);
        info!("");

        // Sync result
        info!("  Sync Result:");
        info!(
            "    Status: {}",
            if report.success { "SUCCESS" } else { "FAILED" }
        );
        info!("    Issues synced this run: {}", report.issues_synced);
        if let Some(last_updated) = report.last_issue_updated_at {
            info!("    Last issue updated_at: {}", last_updated);
        }
        info!(
            "    Duration: {:.1}s",
            self.start_time.elapsed().as_secs_f64()
        );
        info!("");
        info!("===============================================================");
        info!("");
    }
}

/// Logger for a single step
pub struct StepLogger {
    start_time: Instant,
}

impl StepLogger {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }

    pub fn detail(&self, message: &str) {
        info!("  ├─ {}", message);
    }

    pub fn last_detail(&self, message: &str) {
        info!("  └─ {}", message);
    }

    pub fn finish(&self) -> f64 {
        let duration = self.start_time.elapsed().as_secs_f64();
        info!("  └─ Duration: {:.1}s", duration);
        info!("");
        duration
    }

    pub fn finish_with_detail(&self, message: &str) -> f64 {
        let duration = self.start_time.elapsed().as_secs_f64();
        info!("  └─ {} (Duration: {:.1}s)", message, duration);
        info!("");
        duration
    }
}

/// Summary report for sync operation
#[derive(Debug, Default)]
pub struct SyncSummaryReport {
    pub jira_total_count: usize,
    pub jira_status_counts: HashMap<String, usize>,
    pub local_total_count: usize,
    pub local_status_counts: HashMap<String, usize>,
    pub local_history_count: usize,
    pub local_snapshot_count: usize,
    pub issues_synced: usize,
    pub last_issue_updated_at: Option<DateTime<Utc>>,
    pub success: bool,
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:width$}", s, width = max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
