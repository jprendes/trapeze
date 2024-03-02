use std::collections::{BTreeMap, BTreeSet};
use std::ops::DerefMut as _;
use std::sync::Arc;

use tokio::sync::RwLock;

pub struct IdMap<T: Send + Sync> {
    inner: Arc<RwLock<IdMapInner<T>>>,
}

struct IdMapInner<T: Send + Sync> {
    used: BTreeMap<u32, Arc<T>>,
    available: BTreeSet<u32>,
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
        Self {
            used: Default::default(),
            available: Default::default(),
        }
    }
}

impl<T: Send + Sync + 'static> Clone for IdMap<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct IdMapGuard<T: Send + Sync + 'static> {
    id: u32,
    map: IdMap<T>,
}

impl<T: Send + Sync + 'static> IdMap<T> {
    fn claim_impl(&self, inner: &mut IdMapInner<T>, id: u32, value: T) -> Option<IdMapGuard<T>> {
        if inner.used.contains_key(&id) {
            return None;
        }
        let value = Arc::new(value);
        inner.used.insert(id, value.clone());
        inner.available.remove(&id);
        Some(IdMapGuard {
            id,
            map: self.clone(),
        })
    }

    pub async fn claim(&self, id: u32, value: T) -> Option<IdMapGuard<T>> {
        let mut inner = self.inner.write().await;
        self.claim_impl(inner.deref_mut(), id, value)
    }

    pub async fn claim_any(&self, value: T) -> IdMapGuard<T> {
        let mut inner = self.inner.write().await;
        let id = match inner.available.pop_first() {
            Some(id) => id,
            None => inner.used.last_key_value().map(|(k, _)| k + 2).unwrap_or(1),
        };
        self.claim_impl(inner.deref_mut(), id, value).unwrap()
    }

    pub async fn borrow(&self, id: u32) -> Option<Arc<T>> {
        self.inner.read().await.used.get(&id).cloned()
    }
}

impl<T: Send + Sync + 'static> IdMapGuard<T> {
    pub fn id(&self) -> u32 {
        self.id
    }
}

impl<T: Send + Sync + 'static> Drop for IdMapGuard<T> {
    fn drop(&mut self) {
        let id = self.id;
        let map = self.map.clone();
        tokio::spawn(async move {
            let mut inner = map.inner.write().await;
            inner.used.remove(&id);
            inner.available.insert(id);
        });
    }
}
