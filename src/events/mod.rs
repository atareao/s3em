use std::sync::Arc;
use tokio::sync::broadcast;
use crate::db::events::EventRepository;
use crate::models::AppEvent;

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<AppEvent>,
    event_repo: Arc<EventRepository>,
}

impl EventBus {
    pub fn new(buffer: usize, event_repo: Arc<EventRepository>) -> Self {
        let (tx, _) = broadcast::channel(buffer);
        Self { tx, event_repo }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.tx.subscribe()
    }

    pub async fn emit(&self, event: AppEvent) {
        if let Err(e) = self.event_repo.persist(&event).await {
            tracing::error!("Failed to persist event: {e}");
        }
        let _ = self.tx.send(event);
    }
}