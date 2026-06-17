use axum::Json;
use serde_json::Value;

pub async fn health() -> Json<Value> {
    Json(serde_json::json!({"status": "ok"}))
}