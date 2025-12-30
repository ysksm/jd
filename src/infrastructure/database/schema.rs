use duckdb::Connection;
use crate::domain::error::{DomainError, DomainResult};

pub struct Schema;

impl Schema {
    pub fn init(conn: &Connection) -> DomainResult<()> {
        Self::create_projects_table(conn)?;
        Self::create_issues_table(conn)?;
        Self::create_sync_history_table(conn)?;
        Self::create_metadata_tables(conn)?;
        Self::create_change_history_table(conn)?;
        Self::create_indexes(conn)?;
        Ok(())
    }

    fn create_projects_table(conn: &Connection) -> DomainResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id VARCHAR PRIMARY KEY,
                key VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description TEXT,
                raw_data JSON,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create projects table: {}", e)))?;
        Ok(())
    }

    fn create_issues_table(conn: &Connection) -> DomainResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS issues (
                id VARCHAR PRIMARY KEY,
                project_id VARCHAR NOT NULL,
                key VARCHAR NOT NULL,
                summary TEXT NOT NULL,
                description TEXT,
                status VARCHAR,
                priority VARCHAR,
                assignee VARCHAR,
                reporter VARCHAR,
                issue_type VARCHAR,
                resolution VARCHAR,
                labels VARCHAR,
                components VARCHAR,
                fix_versions VARCHAR,
                parent_key VARCHAR,
                created_date TIMESTAMP,
                updated_date TIMESTAMP,
                raw_data JSON,
                synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create issues table: {}", e)))?;
        Ok(())
    }

    fn create_sync_history_table(conn: &Connection) -> DomainResult<()> {
        conn.execute(
            "CREATE SEQUENCE IF NOT EXISTS sync_history_id_seq START 1",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create sync_history sequence: {}", e)))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS sync_history (
                id INTEGER PRIMARY KEY DEFAULT nextval('sync_history_id_seq'),
                project_id VARCHAR NOT NULL,
                sync_type VARCHAR NOT NULL,
                started_at TIMESTAMP NOT NULL,
                completed_at TIMESTAMP,
                status VARCHAR NOT NULL,
                items_synced INTEGER,
                error_message TEXT
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create sync_history table: {}", e)))?;
        Ok(())
    }

    fn create_metadata_tables(conn: &Connection) -> DomainResult<()> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS statuses (
                project_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description VARCHAR,
                category VARCHAR,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                PRIMARY KEY (project_id, name)
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create statuses table: {}", e)))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS priorities (
                project_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description VARCHAR,
                icon_url VARCHAR,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                PRIMARY KEY (project_id, name)
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create priorities table: {}", e)))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS issue_types (
                project_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description VARCHAR,
                icon_url VARCHAR,
                subtask BOOLEAN DEFAULT false,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                PRIMARY KEY (project_id, name)
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create issue_types table: {}", e)))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS labels (
                project_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                PRIMARY KEY (project_id, name)
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create labels table: {}", e)))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS components (
                project_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description VARCHAR,
                lead VARCHAR,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                PRIMARY KEY (project_id, name)
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create components table: {}", e)))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS fix_versions (
                project_id VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                description VARCHAR,
                released BOOLEAN DEFAULT false,
                release_date TIMESTAMP,
                created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP NOT NULL,
                PRIMARY KEY (project_id, name)
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create fix_versions table: {}", e)))?;

        Ok(())
    }

    fn create_change_history_table(conn: &Connection) -> DomainResult<()> {
        conn.execute(
            "CREATE SEQUENCE IF NOT EXISTS change_history_id_seq START 1",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create change_history sequence: {}", e)))?;

        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS issue_change_history (
                id INTEGER PRIMARY KEY DEFAULT nextval('change_history_id_seq'),
                issue_id VARCHAR NOT NULL,
                issue_key VARCHAR NOT NULL,
                history_id VARCHAR NOT NULL,
                author_account_id VARCHAR,
                author_display_name VARCHAR,
                field VARCHAR NOT NULL,
                field_type VARCHAR,
                from_value VARCHAR,
                from_string VARCHAR,
                to_value VARCHAR,
                to_string VARCHAR,
                changed_at TIMESTAMP NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create issue_change_history table: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_change_history_issue_id ON issue_change_history(issue_id)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_change_history_issue_key ON issue_change_history(issue_key)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_change_history_field ON issue_change_history(field)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_change_history_changed_at ON issue_change_history(changed_at)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;

        Ok(())
    }

    fn create_indexes(conn: &Connection) -> DomainResult<()> {
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_issues_project ON issues(project_id)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_issues_key ON issues(key)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sync_history_project ON sync_history(project_id)",
            [],
        ).map_err(|e| DomainError::Repository(format!("Failed to create index: {}", e)))?;
        Ok(())
    }
}
