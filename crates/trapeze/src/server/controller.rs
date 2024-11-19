use std::future::Future;

use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
pub struct ServerController {
    pub(super) shutdown: CancellationToken,
    pub(super) abort: CancellationToken,
}

impl ServerController {
    pub fn terminate(&self) {
        self.abort.cancel();
    }

    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    pub fn new() -> Self {
        Self::default()
    }
}

impl ServerController {
    pub(super) fn control<F: Future + Send>(
        &self,
        fut_fn: impl FnOnce() -> F,
    ) -> impl Future<Output = Option<F::Output>> + Send {
        let abort = self.abort.clone();
        let task = fut_fn();
        async move {
            tokio::select! {
                () = abort.cancelled() => { None },
                val = task => { Some(val) },
            }
        }
    }
}
