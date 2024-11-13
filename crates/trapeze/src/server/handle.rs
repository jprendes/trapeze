use std::future::Future;
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::ops::Deref;
use std::sync::Arc;
use std::task::{ready, Poll};

use futures::future::Aborted;
use futures::FutureExt as _;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::server::controller::ServerController;

pub struct ServerHandle {
    pub(super) controller: ServerController,
    pub(super) handle: JoinHandle<Result<IoResult<()>, Aborted>>,
}

impl ServerHandle {
    pub fn controller(&self) -> ServerController {
        self.controller.clone()
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.controller.terminate();
    }
}

impl Deref for ServerHandle {
    type Target = ServerController;

    fn deref(&self) -> &Self::Target {
        &self.controller
    }
}

impl Future for ServerHandle {
    type Output = IoResult<()>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match ready!(self.handle.poll_unpin(cx)) {
            Ok(Ok(res)) => Poll::Ready(res),
            _ => Poll::Ready(Err(IoError::new(
                ErrorKind::Interrupted,
                "TTRPC server terminated abruptly",
            ))),
        }
    }
}

impl ServerHandle {
    pub(super) fn spawn<F: Future<Output = IoResult<()>> + Send + 'static>(
        fut_fn: impl Send + 'static + FnOnce(Arc<Notify>) -> F,
    ) -> Self {
        let (controller, task) = ServerController::control(fut_fn);
        let handle = tokio::spawn(task);
        ServerHandle { controller, handle }
    }
}
