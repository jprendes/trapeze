use std::collections::BTreeMap;

// Use `flume`'s `unbounded` channel instead of `std`'s channel as they implement `Sync`
//use flume::{unbounded, Receiver, Sender};

// Use `tokio`'s `unbounded_channel` channel instead of `std`'s channel as they implement `Sync`
use tokio::sync::mpsc::{
    unbounded_channel as unbounded, UnboundedReceiver as Receiver, UnboundedSender as Sender,
};

use self::odd_range_pool::OddRangePool;

mod odd_range_pool;
mod range_pool;

pub struct IdPool<T: Send + Sync> {
    used: BTreeMap<u32, T>,
    free: OddRangePool,

    tx: Sender<u32>,
    rx: Receiver<u32>,
}

pub struct IdPoolGuard {
    id: u32,
    tx: Sender<u32>,
}

impl<T: Send + Sync> Default for IdPool<T> {
    fn default() -> Self {
        let (tx, rx) = unbounded();
        Self {
            used: Default::default(),
            free: Default::default(),
            tx,
            rx,
        }
    }
}

impl<T: Send + Sync> IdPool<T> {
    fn recycle(&mut self) {
        while let Ok(id) = self.rx.try_recv() {
            self.used.remove(&id);
            let _ = self.free.return_id(id);
        }
    }

    pub fn claim(&mut self, id: impl Into<Option<u32>>, value: T) -> Option<IdPoolGuard> {
        self.recycle();
        let id = match id.into() {
            Some(id) => self.free.request_id(id)?,
            None => self.free.new_id()?,
        };
        self.used.insert(id, value);
        Some(IdPoolGuard {
            id,
            tx: self.tx.clone(),
        })
    }

    pub fn get(&mut self, id: u32) -> Option<&mut T> {
        self.recycle();
        self.used.get_mut(&id)
    }
}

impl<T: Send + Sync> Drop for IdPool<T> {
    fn drop(&mut self) {
        self.recycle();
    }
}

impl IdPoolGuard {
    pub fn id(&self) -> u32 {
        self.id
    }
}

impl Drop for IdPoolGuard {
    fn drop(&mut self) {
        let _ = self.tx.send(self.id);
    }
}
