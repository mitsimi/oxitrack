use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::db::DbPool;

mod db;

const PORT: u16 = 3000;

#[tokio::main]
async fn main() {
    let db = match DbPool::new("oxitrack.db").await {
        Ok(db) => db,
        Err(err) => {
            eprintln!("Failed to initialize database: {}", err);
            std::process::exit(1);
        }
    };

    let app = Router::new().route("/beat", post(heartbeat)).with_state(db);

    // run our app with hyper, listening globally on port 3000
    let listener = match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", PORT)).await {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("Failed to bind to port {}: {}", PORT, err);
            std::process::exit(1);
        }
    };

    println!("Listening on http://localhost:{}", PORT);
    match axum::serve(listener, app).await {
        Ok(_) => (),
        Err(err) => {
            eprintln!("Failed to serve: {}", err);
            std::process::exit(1);
        }
    };
}

#[derive(Debug, Deserialize)]
struct HeartbeatRequest {
    project_handle: String,
    timestamp: i64,
}

#[derive(Debug, Serialize)]
struct HeartbeatResponse {
    session_id: i64,
    project_handle: String,
    duration_seconds: i64,
}

async fn heartbeat(
    State(db): State<DbPool>,
    Json(request): Json<HeartbeatRequest>,
) -> Result<Json<HeartbeatResponse>, Json<Value>> {
    println!("received heartbeat for {}", request.project_handle);

    if request.project_handle.len() > 100 {
        eprintln!("project handle exeeds 100 character limit");
        return Err(Json(json!({
            "error": "project_handle exceeds 100 character limit"
        })));
    }

    match db
        .update_session(&request.project_handle, request.timestamp)
        .await
    {
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
