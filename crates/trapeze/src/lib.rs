mod client;
mod context;
mod id_pool;
mod io;
mod server;
mod service;
mod types;

pub type Result<T, E = Status> = std::result::Result<T, E>;

pub use client::Client;
pub use context::metadata::Metadata;
pub use context::timeout::Timeout;
pub use context::{get_context, try_get_context, Context};
pub use server::Server;
pub use trapeze_codegen as codegen;
pub use trapeze_macros::*;
pub use types::protos::{Code, Status};

pub mod __codegen_prelude {
    pub use crate::client::request_handlers::RequestHandler;
    pub use crate::server::method_handlers::MethodHandler;
    pub use crate::service::{
        ClientStreamingMethod, DuplexStreamingMethod, MethodNotFound, ServerStreamingMethod,
        Service, UnaryMethod,
    };
}

pub mod prelude {
    pub use std::future::Future;

    pub use futures::stream::Stream;

    pub use crate::Result;
}

pub mod stream {
    pub use async_stream::{stream, try_stream};
}
