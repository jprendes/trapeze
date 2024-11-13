use std::fmt::Debug;
use std::future::Future;
use std::ops::Deref;
use std::sync::Arc;

pub mod metadata;
pub mod timeout;

use metadata::Metadata;
use timeout::Timeout;
use tokio::task::futures::TaskLocalFuture;

use crate::ServerController;

#[derive(Default, Clone, Debug)]
pub struct Context {
    pub metadata: Metadata,
    pub timeout: Timeout,
}

#[derive(Clone)]
pub struct ServerContext {
    pub server: Option<ServerController>,
    context: Arc<Context>,
}

impl Debug for ServerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.context.fmt(f)
    }
}

impl Deref for ServerContext {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

tokio::task_local! {
    static CONTEXT: ServerContext;
}

#[must_use]
pub fn get_context() -> ServerContext {
    CONTEXT.with(Clone::clone)
}

#[must_use]
pub fn try_get_context() -> Option<ServerContext> {
    CONTEXT.try_with(Clone::clone).ok()
}

#[must_use]
pub fn get_server() -> ServerController {
    get_context().server.unwrap()
}

#[must_use]
pub fn try_get_server() -> Option<ServerController> {
    try_get_context().map(|ctx| ctx.server).flatten()
}

pub trait WithContext: Future {
    fn with_context(
        self,
        ctx: impl Into<Arc<Context>>,
        controller: impl Into<Option<ServerController>>,
    ) -> TaskLocalFuture<ServerContext, Self>
    where
        Self: Sized,
    {
        CONTEXT.scope(
            ServerContext {
                server: controller.into(),
                context: ctx.into(),
            },
            self,
        )
    }
}

impl<F: Future> WithContext for F {}
