use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::RwLock;

pub struct IdMap<T: Send + Sync> {
    inner: Arc<RwLock<IdMapInner<T>>>,
}

struct IdMapInner<T: Send + Sync> {
    used: BTreeMap<u32, Arc<T>>,
    available: BTreeSet<u32>,
    rx: UnboundedReceiver<u32>,
    tx: UnboundedSender<u32>,
}

pub struct IdMapGuard {
    id: u32,
    tx: UnboundedSender<u32>,
}

impl<T: Send + Sync> Default for IdMap<T> {
    fn default() -> Self {
        Self {
            inner: Arc::default(),
        }
    }
}

impl<T: Send + Sync> Default for IdMapInner<T> {
    fn default() -> Self {
        let (tx, rx) = unbounded_channel();
        Self {
            used: Default::default(),
            available: Default::default(),
            rx,
            tx,
        }
    }
}

impl<T: Send + Sync> Clone for IdMap<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Send + Sync> IdMap<T> {
    pub async fn claim(&self, id: impl Into<Option<u32>>, value: T) -> Option<IdMapGuard> {
        self.inner.write().await.claim(id.into(), value)
    }

    pub async fn claim_any(&self, value: T) -> IdMapGuard {
        // safe because "id: None" means we will always get "Some(guard)" back.
        self.claim(None, value).await.unwrap()
    }

    pub async fn borrow(&self, id: u32) -> Option<Arc<T>> {
        self.inner.read().await.borrow(id)
    }
}

impl<T: Send + Sync> IdMapInner<T> {
    fn get_any_id(&mut self) -> u32 {
        if let Some(id) = self.available.pop_first() {
            id
        } else if let Some((id, _)) = self.used.last_key_value() {
            id + 2
        } else {
            1
        }
    }

    fn recycle(&mut self) {
        while let Ok(id) = self.rx.try_recv() {
            self.used.remove(&id);
            self.available.insert(id);
        }
    }

    pub fn claim(&mut self, id: Option<u32>, value: T) -> Option<IdMapGuard> {
        self.recycle();
        let id = match id {
            Some(id) => id,
            None => self.get_any_id(),
        };
        if self.used.contains_key(&id) {
            return None;
        }
        let value = Arc::new(value);
        self.used.insert(id, value.clone());
        self.available.remove(&id);
        Some(IdMapGuard {
            id,
            tx: self.tx.clone(),
        })
    }

    pub fn borrow(&self, id: u32) -> Option<Arc<T>> {
        self.used.get(&id).cloned()
    }
}

impl IdMapGuard {
    pub fn id(&self) -> u32 {
        self.id
    }
}

impl Drop for IdMapGuard {
    fn drop(&mut self) {
        let _ = self.tx.send(self.id);
    }
}
