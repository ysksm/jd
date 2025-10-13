use crate::error::Result;
use duckdb::Connection;

pub struct Schema;

impl Schema {
    /// Initialize database schema
    pub fn initialize(conn: &Connection) -> Result<()> {
        Self::create_projects_table(conn)?;
        Self::create_issues_table(conn)?;
        Self::create_sync_history_table(conn)?;
        Self::create_metadata_tables(conn)?;
        Self::create_indexes(conn)?;
        Ok(())
    }

    fn create_projects_table(conn: &Connection) -> Result<()> {
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
        )?;
        Ok(())
    }

    fn create_issues_table(conn: &Connection) -> Result<()> {
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
        )?;
        Ok(())
    }

    fn create_sync_history_table(conn: &Connection) -> Result<()> {
        // Create sequence for auto-incrementing ID
        conn.execute(
            "CREATE SEQUENCE IF NOT EXISTS sync_history_id_seq START 1",
            [],
        )?;

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
        )?;
        Ok(())
    }

    fn create_metadata_tables(conn: &Connection) -> Result<()> {
        // Statuses table
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
        )?;

        // Priorities table
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
        )?;

        // Issue types table
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
        )?;

        // Labels table
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
        )?;

        // Components table
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
        )?;

        // Fix versions table
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
        )?;

        Ok(())
    }

    fn create_indexes(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_issues_project ON issues(project_id)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_issues_key ON issues(key)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sync_history_project ON sync_history(project_id)",
            [],
        )?;
        Ok(())
    }
}
