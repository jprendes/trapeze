use std::future::Future;
use std::sync::Arc;

pub mod metadata;
pub mod timeout;

use metadata::Metadata;
use timeout::Timeout;
use tokio::task::futures::TaskLocalFuture;

#[derive(Default, Clone, Debug)]
pub struct Context {
    pub metadata: Metadata,
    pub timeout: Timeout,
}

tokio::task_local! {
    static CONTEXT: Arc<Context>;
}

pub fn get_context() -> Arc<Context> {
    CONTEXT.with(|c| c.clone())
}

pub fn try_get_context() -> Option<Arc<Context>> {
    CONTEXT.try_with(|c| c.clone()).ok()
}

pub trait WithContext: Future {
    fn with_context(self, ctx: impl Into<Arc<Context>>) -> TaskLocalFuture<Arc<Context>, Self>
    where
        Self: Sized,
    {
        CONTEXT.scope(ctx.into(), self)
    }
}

impl<F: Future> WithContext for F {}
