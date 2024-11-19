use std::future::Future;
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::ops::Deref;
use std::task::{ready, Poll};

use futures::FutureExt as _;
use tokio::task::JoinHandle;

use crate::server::controller::ServerController;

pub struct ServerHandle {
    pub(super) controller: ServerController,
    pub(super) handle: JoinHandle<Option<IoResult<()>>>,
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
            Ok(Some(res)) => Poll::Ready(res),
            _ => Poll::Ready(Err(IoError::new(
                ErrorKind::Interrupted,
                "TTRPC server terminated abruptly",
            ))),
        }
    }
}

impl ServerHandle {
    pub fn new() -> Self {
        Self::spawn(|_| async { Ok(()) })
    }

    pub(super) fn spawn<F: Future<Output = IoResult<()>> + Send + 'static>(
        fut_fn: impl Send + 'static + FnOnce(ServerController) -> F,
    ) -> Self {
        let controller = ServerController::new();
        let task = controller.control({
            let controller = controller.clone();
            move || {
                fut_fn(controller)
            }
        });
        let handle = tokio::spawn(task);
        ServerHandle { controller, handle }
    }
}

impl Default for ServerHandle {
    fn default() -> Self {
        Self::new()
    }
}
