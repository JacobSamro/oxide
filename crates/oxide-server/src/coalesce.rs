use std::future::Future;
use std::hash::Hash;
use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::broadcast;

/// Singleflight: only one in-flight call per key; followers receive the same result.
///
/// Why broadcast over Notify+shared slot: callers can race a join and a removal otherwise; a
/// broadcast channel cleanly fans out a single computation to N waiters even if some arrive
/// after the producer finishes (we keep capacity for late subscribers).
pub struct Singleflight<K, V> {
    inflight: Arc<DashMap<K, broadcast::Sender<Arc<V>>>>,
}

impl<K, V> Default for Singleflight<K, V>
where K: Eq + Hash + Clone, V: Send + Sync + 'static
{
    fn default() -> Self { Self { inflight: Arc::new(DashMap::new()) } }
}

impl<K, V> Singleflight<K, V>
where K: Eq + Hash + Clone, V: Send + Sync + 'static
{
    pub fn new() -> Self { Self::default() }

    /// Returns (value, coalesced) where coalesced=true means we waited on another caller's work.
    pub async fn run<F, Fut>(&self, key: K, f: F) -> (Arc<V>, bool)
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = V>,
    {
        // Try to register as the producer.
        let (tx, _is_producer) = match self.inflight.entry(key.clone()) {
            dashmap::mapref::entry::Entry::Occupied(e) => {
                // Subscribe before releasing the entry to avoid missing the broadcast.
                let mut rx = e.get().subscribe();
                drop(e);
                if let Ok(v) = rx.recv().await { return (v, true); }
                // Sender dropped without sending — fall through and retry as producer.
                return Box::pin(self.run(key, f)).await;
            }
            dashmap::mapref::entry::Entry::Vacant(v) => {
                let (tx, _) = broadcast::channel(8);
                v.insert(tx.clone());
                (tx, true)
            }
        };

        let value = Arc::new(f().await);
        let _ = tx.send(value.clone());
        self.inflight.remove(&key);
        (value, false)
    }
}
