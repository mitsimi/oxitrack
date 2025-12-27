use anyhow::Result;
use sqlx::{FromRow, sqlite::SqlitePool};

#[derive(Debug, Clone)]
pub struct DbPool(SqlitePool);

#[derive(Debug, FromRow)]
struct SessionRow {
    id: i64,
    start_time: i64,
}

impl DbPool {
    pub async fn new(db_path: &str) -> Result<Self> {
        let pool = match SqlitePool::connect(db_path).await {
            Ok(pool) => pool,
            Err(_) => {
                // Create a new database file if it doesn't exist
                let path = std::path::Path::new(db_path);
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::File::create(path)?;
                SqlitePool::connect(db_path).await?
            }
        };
        Self::init_db(&pool).await?;
        Ok(Self(pool))
    }

    async fn init_db(pool: &SqlitePool) -> Result<()> {
        // Create the sessions table if it doesn't exist
        // project_handle should only have max 100 chars
        // start_time and last_heartbeat are unix timestamps
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY,
                project_handle TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                last_heartbeat INTEGER NOT NULL,
                UNIQUE(project_handle, start_time) ON CONFLICT REPLACE
            )",
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_session(self, project_handle: &str, timestamp: i64) -> Result<(i64, i64)> {
        let five_minutes_ago = timestamp - (5 * 60);

        let tx = self.0.begin().await?;

        let session = sqlx::query_as::<_, SessionRow>(
            "SELECT id, start_time FROM sessions
             WHERE project_handle = ? AND last_heartbeat > ?
             ORDER BY last_heartbeat DESC LIMIT 1",
        )
        .bind(project_handle)
        .bind(five_minutes_ago)
        .fetch_optional(&self.0)
        .await?;

        let session = match session {
            Some(s) => {
                sqlx::query("UPDATE sessions SET last_heartbeat = ? WHERE id = ?")
                    .bind(timestamp)
                    .bind(s.id)
                    .execute(&self.0)
                    .await?;

                Ok((s.id, timestamp - s.start_time))
            }
            None => {
                let result = sqlx::query(
                    "INSERT INTO sessions (project_handle, start_time, last_heartbeat) VALUES (?, ?, ?)",
                )
                .bind(project_handle)
                .bind(timestamp)
                .bind(timestamp)
                .execute(&self.0)
                .await?;

                Ok((result.last_insert_rowid(), 0))
            }
        };

        tx.commit().await?;

        session
    }
}
