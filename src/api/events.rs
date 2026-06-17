use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
    routing::get,
    Json, Router,
};
use futures_util::stream::Stream;
use std::convert::Infallible;
use tokio::sync::broadcast;

use crate::api::SharedState;
use crate::db;
use crate::error::AppError;
use crate::models::{EventHistoryFilter, EventRecord};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(event_stream))
        .route("/history", get(event_history))
}

async fn event_stream(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_bus.subscribe();

    let stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap();
                    yield Ok(Event::default().data(data));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Sse::new(stream)
}

async fn event_history(
    State(state): State<SharedState>,
    Query(filters): Query<EventHistoryFilter>,
) -> Result<Json<Vec<EventRecord>>, AppError> {
    let repo = db::events::EventRepository::new(state.pool.clone());
    let events = repo.list_events(&state.pool, &filters)?;
    Ok(Json(events))
}