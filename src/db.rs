use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

type Result<T> = std::result::Result<T, sqlx::Error>;

#[derive(Debug, sqlx::FromRow)]
struct SessionRow {
    id: i64,
    start_time: i64,
}

pub async fn create_pool(database_url: &str) -> Result<SqlitePool> {
    let pool = match SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
    {
        Ok(pool) => pool,
        Err(_) => {
            // Create a new database file if it doesn't exist
            let path = std::path::Path::new(database_url);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::File::create(path)?;
            SqlitePool::connect(database_url).await?
        }
    };

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // Create the sessions table if it doesn't exist
    // project_handle should only have max 100 chars
    // start_time and last_heartbeat are unix timestamps
    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY,
                project_handle TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                last_heartbeat INTEGER NOT NULL,
                UNIQUE(project_handle, start_time) ON CONFLICT REPLACE
                )
            "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_session(
    pool: &SqlitePool,
    project_handle: &str,
    timestamp: i64,
) -> Result<(i64, i64)> {
    let five_minutes_ago = timestamp - (5 * 60);

    let tx = pool.begin().await?;

    let session = sqlx::query_as::<_, SessionRow>(
        "SELECT id, start_time FROM sessions
             WHERE project_handle = ? AND last_heartbeat > ?
             ORDER BY last_heartbeat DESC LIMIT 1",
    )
    .bind(project_handle)
    .bind(five_minutes_ago)
    .fetch_optional(pool)
    .await?;

    let session = match session {
        Some(s) => {
            sqlx::query("UPDATE sessions SET last_heartbeat = ? WHERE id = ?")
                .bind(timestamp)
                .bind(s.id)
                .execute(pool)
                .await?;

            Ok((s.id, timestamp - s.start_time))
        }
        None => {
            close_stale_session(pool, project_handle).await?;

            let result = sqlx::query(
                    "INSERT INTO sessions (project_handle, start_time, last_heartbeat) VALUES (?, ?, ?)",
                )
                .bind(project_handle)
                .bind(timestamp)
                .bind(timestamp)
                .execute(pool)
                .await?;

            Ok((result.last_insert_rowid(), 0))
        }
    };

    tx.commit().await?;

    session
}

async fn close_stale_session(pool: &SqlitePool, project_handle: &str) -> Result<()> {
    sqlx::query(
        "UPDATE sessions SET end_time = last_heartbeat
            WHERE project_handle = ? AND end_time IS NULL",
    )
    .bind(project_handle)
    .execute(pool)
    .await?;

    Ok(())
}
