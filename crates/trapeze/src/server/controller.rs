use std::future::Future;
use std::sync::Arc;

use futures::future::{abortable, AbortHandle};
use futures::stream::Aborted;
use tokio::sync::Notify;
use tokio::task::futures::TaskLocalFuture;

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

tokio::task_local! {
    static CONTROLLER: Option<ServerController>;
}

impl ServerController {
    pub(super) fn control<F: Future + Send>(
        fut_fn: impl Send + FnOnce(Arc<Notify>) -> F,
    ) -> (
        Self,
        impl Future<Output = Result<F::Output, Aborted>> + Send,
    ) {
        let shutdown = Arc::new(Notify::new());
        let (task, abort_handle) = abortable(fut_fn(shutdown.clone()));
        let controller = ServerController {
            shutdown,
            abort_handle,
        };
        let task = task.with_server(controller.clone());
        (controller, task)
    }
}

#[must_use]
pub fn get_server() -> ServerController {
    CONTROLLER.with(Clone::clone).unwrap()
}

#[must_use]
pub fn try_get_server() -> Option<ServerController> {
    CONTROLLER.try_with(Clone::clone).ok().flatten()
}

pub(crate) trait WithServerController: Future {
    fn with_server(
        self,
        controller: impl Into<Option<ServerController>>,
    ) -> TaskLocalFuture<Option<ServerController>, Self>
    where
        Self: Sized,
    {
        CONTROLLER.scope(controller.into(), self)
    }

    fn inherit_server(self) -> TaskLocalFuture<Option<ServerController>, Self>
    where
        Self: Sized,
    {
        self.with_server(try_get_server())
    }
}

impl<F: Future> WithServerController for F {}
