//! Repository for issue embeddings
//!
//! Provides storage and retrieval of vector embeddings for semantic search.

use duckdb::Connection;
use std::sync::{Arc, Mutex};

use crate::domain::error::{DomainError, DomainResult};

/// Represents an issue with its embedding
#[derive(Debug, Clone)]
pub struct IssueEmbedding {
    /// Issue ID
    pub issue_id: String,
    /// Issue key (e.g., "PROJ-123")
    pub issue_key: String,
    /// The embedding vector
    pub embedding: Vec<f32>,
    /// The text that was embedded
    pub embedded_text: String,
    /// Embedding provider (e.g., "openai", "ollama", "cohere")
    pub provider: String,
    /// Model name (e.g., "text-embedding-3-small", "nomic-embed-text")
    pub model: String,
    /// Embedding dimensions
    pub dimensions: i32,
    /// Timestamp when the embedding was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Search result with similarity score
#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    /// Issue key
    pub issue_key: String,
    /// Issue summary
    pub summary: String,
    /// Issue description
    pub description: Option<String>,
    /// Status
    pub status: Option<String>,
    /// Project ID
    pub project_id: String,
    /// Similarity score (lower is more similar for distance metrics)
    pub similarity_score: f32,
}

/// Repository for managing issue embeddings
pub struct EmbeddingsRepository {
    conn: Arc<Mutex<Connection>>,
}

impl EmbeddingsRepository {
    /// Create a new embeddings repository
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Initialize the embeddings table and load VSS extension
    pub fn init_schema(&self) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connection lock: {}", e))
        })?;

        // Install and load VSS extension
        conn.execute("INSTALL vss", []).map_err(|e| {
            DomainError::Repository(format!("Failed to install VSS extension: {}", e))
        })?;
        conn.execute("LOAD vss", [])
            .map_err(|e| DomainError::Repository(format!("Failed to load VSS extension: {}", e)))?;

        // Enable experimental persistence for HNSW index
        conn.execute("SET hnsw_enable_experimental_persistence = true", [])
            .map_err(|e| {
                DomainError::Repository(format!("Failed to enable HNSW persistence: {}", e))
            })?;

        // Create embeddings table with ARRAY type for vectors
        // Using FLOAT[] (variable length) to support different embedding providers
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS issue_embeddings (
                issue_id VARCHAR PRIMARY KEY,
                issue_key VARCHAR NOT NULL,
                embedding FLOAT[] NOT NULL,
                embedded_text TEXT NOT NULL,
                provider VARCHAR NOT NULL DEFAULT 'openai',
                model VARCHAR NOT NULL DEFAULT 'text-embedding-3-small',
                dimensions INTEGER NOT NULL DEFAULT 1536,
                created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )
        .map_err(|e| {
            DomainError::Repository(format!("Failed to create issue_embeddings table: {}", e))
        })?;

        // Create HNSW index for fast similarity search
        // Use cosine metric which is most appropriate for text embeddings
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_embeddings_hnsw
            ON issue_embeddings
            USING HNSW (embedding)
            WITH (metric = 'cosine')
            "#,
            [],
        )
        .map_err(|e| DomainError::Repository(format!("Failed to create HNSW index: {}", e)))?;

        log::info!("Initialized embeddings schema with VSS extension");
        Ok(())
    }

    /// Insert or update an embedding for an issue
    pub fn upsert_embedding(
        &self,
        issue_id: &str,
        issue_key: &str,
        embedding: &[f32],
        embedded_text: &str,
        provider: &str,
        model: &str,
    ) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connection lock: {}", e))
        })?;

        // Convert embedding to array string format for DuckDB
        let embedding_str = format!(
            "[{}]",
            embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let dimensions = embedding.len() as i32;

        conn.execute(
            r#"
            INSERT INTO issue_embeddings (issue_id, issue_key, embedding, embedded_text, provider, model, dimensions, created_at)
            VALUES (?, ?, ?::FLOAT[], ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT (issue_id) DO UPDATE SET
                issue_key = excluded.issue_key,
                embedding = excluded.embedding,
                embedded_text = excluded.embedded_text,
                provider = excluded.provider,
                model = excluded.model,
                dimensions = excluded.dimensions,
                created_at = CURRENT_TIMESTAMP
            "#,
            duckdb::params![issue_id, issue_key, embedding_str, embedded_text, provider, model, dimensions],
        )
        .map_err(|e| DomainError::Repository(format!("Failed to upsert embedding: {}", e)))?;

        Ok(())
    }

    /// Perform semantic search using the query embedding
    pub fn semantic_search(
        &self,
        query_embedding: &[f32],
        project_filter: Option<&str>,
        limit: usize,
    ) -> DomainResult<Vec<SemanticSearchResult>> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connection lock: {}", e))
        })?;

        // Convert query embedding to array string format
        let embedding_str = format!(
            "[{}]",
            query_embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // Build the query with optional project filter
        // Using FLOAT[] cast to support variable-length embeddings from different providers
        let query = match project_filter {
            Some(_) => {
                r#"
                SELECT
                    e.issue_key,
                    i.summary,
                    i.description,
                    i.status,
                    i.project_id,
                    array_cosine_distance(e.embedding, ?::FLOAT[]) as distance
                FROM issue_embeddings e
                JOIN issues i ON e.issue_id = i.id
                WHERE i.project_id = ?
                ORDER BY distance ASC
                LIMIT ?
                "#
            }
            None => {
                r#"
                SELECT
                    e.issue_key,
                    i.summary,
                    i.description,
                    i.status,
                    i.project_id,
                    array_cosine_distance(e.embedding, ?::FLOAT[]) as distance
                FROM issue_embeddings e
                JOIN issues i ON e.issue_id = i.id
                ORDER BY distance ASC
                LIMIT ?
                "#
            }
        };

        let mut stmt = conn.prepare(&query).map_err(|e| {
            DomainError::Repository(format!("Failed to prepare semantic search query: {}", e))
        })?;

        // Helper closure to parse rows
        fn parse_row(row: &duckdb::Row) -> Result<SemanticSearchResult, duckdb::Error> {
            Ok(SemanticSearchResult {
                issue_key: row.get(0)?,
                summary: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                project_id: row.get(4)?,
                similarity_score: row.get(5)?,
            })
        }

        let mut rows = match project_filter {
            Some(project) => stmt.query(duckdb::params![embedding_str, project, limit as i64]),
            None => stmt.query(duckdb::params![embedding_str, limit as i64]),
        }
        .map_err(|e| {
            DomainError::Repository(format!("Failed to execute semantic search: {}", e))
        })?;

        let mut search_results = Vec::new();
        while let Some(row) = rows.next().map_err(|e| {
            DomainError::Repository(format!("Failed to read search result row: {}", e))
        })? {
            match parse_row(row) {
                Ok(r) => search_results.push(r),
                Err(e) => {
                    log::warn!("Error parsing search result row: {}", e);
                }
            }
        }

        Ok(search_results)
    }

    /// Get the count of embeddings in the database
    pub fn count(&self) -> DomainResult<usize> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connection lock: {}", e))
        })?;

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM issue_embeddings", [], |row| {
                row.get(0)
            })
            .map_err(|e| DomainError::Repository(format!("Failed to count embeddings: {}", e)))?;

        Ok(count as usize)
    }

    /// Check if an embedding exists for an issue
    pub fn exists(&self, issue_id: &str) -> DomainResult<bool> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connection lock: {}", e))
        })?;

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM issue_embeddings WHERE issue_id = ?",
                [issue_id],
                |row| row.get(0),
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to check embedding existence: {}", e))
            })?;

        Ok(count > 0)
    }

    /// Delete embedding for an issue
    pub fn delete(&self, issue_id: &str) -> DomainResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connection lock: {}", e))
        })?;

        conn.execute(
            "DELETE FROM issue_embeddings WHERE issue_id = ?",
            [issue_id],
        )
        .map_err(|e| DomainError::Repository(format!("Failed to delete embedding: {}", e)))?;

        Ok(())
    }

    /// Delete all embeddings for a project
    pub fn delete_by_project(&self, project_id: &str) -> DomainResult<usize> {
        let conn = self.conn.lock().map_err(|e| {
            DomainError::Repository(format!("Failed to acquire connection lock: {}", e))
        })?;

        let count = conn
            .execute(
                r#"
                DELETE FROM issue_embeddings
                WHERE issue_id IN (
                    SELECT id FROM issues WHERE project_id = ?
                )
                "#,
                [project_id],
            )
            .map_err(|e| {
                DomainError::Repository(format!("Failed to delete embeddings by project: {}", e))
            })?;

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_embedding_conversion() {
        // Test that embedding conversion to string format works correctly
        let embedding = vec![0.1, 0.2, 0.3, -0.4, 0.5];
        let embedding_str = format!(
            "[{}]",
            embedding
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        assert!(embedding_str.contains("0.1"));
        assert!(embedding_str.contains("-0.4"));
        assert!(embedding_str.starts_with('[') && embedding_str.ends_with(']'));
    }
}
