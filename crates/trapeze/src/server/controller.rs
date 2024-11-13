use std::future::Future;
use std::sync::Arc;

use futures::future::{abortable, AbortHandle};
use futures::stream::Aborted;
use tokio::sync::{oneshot, Notify};

#[derive(Clone)]
pub struct ServerController {
    pub(super) shutdown: Arc<Notify>,
    pub(super) abort_handle: AbortHandle,
}

impl ServerController {
    pub fn terminate(&self) {
        self.abort_handle.abort();
    }

    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
    }
}

impl ServerController {
    pub(super) fn control<F: Future + Send>(
        fut_fn: impl Send + FnOnce(ServerController, Arc<Notify>) -> F,
    ) -> (
        Self,
        impl Future<Output = Result<F::Output, Aborted>> + Send,
    ) {
        let shutdown = Arc::new(Notify::new());
        let (tx, rx) = oneshot::channel();
        let task = {
            let shutdown = shutdown.clone();
            async move {
                let controller = rx.await.unwrap();
                fut_fn(controller, shutdown).await
            }
        };
        let (task, abort_handle) = abortable(task);
        let controller = ServerController {
            shutdown,
            abort_handle,
        };
        let _ = tx.send(controller.clone());
        (controller, task)
    }
}
