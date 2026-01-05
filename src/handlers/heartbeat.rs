use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::db;

#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    project_handle: String,
    timestamp: i64,
}

#[derive(Debug, Serialize)]
pub struct HeartbeatResponse {
    session_id: i64,
    project_handle: String,
    duration_seconds: i64,
}

pub async fn beat(
    State(db): State<SqlitePool>,
    Json(request): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, Json<Value>> {
    println!("received heartbeat for {}", request.project_handle);

    if request.project_handle.len() > 100 {
        eprintln!("project handle exeeds 100 character limit");
        return Err(Json(json!({
            "error": "project_handle exceeds 100 character limit"
        })));
    }

    match db::update_session(&db, &request.project_handle, request.timestamp).await {
        Ok((row_id, time)) => Ok(Json(HeartbeatResponse {
            session_id: row_id,
            project_handle: request.project_handle,
            duration_seconds: time,
        })),
        Err(err) => Err(Json(json!({
            "error": err.to_string()
        }))),
    }
}
