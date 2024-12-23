mod client;
mod context;
mod id_pool;
mod io;
mod server;
mod service;
pub mod transport;
mod types;

pub type Result<T, E = Status> = std::result::Result<T, E>;

pub use client::{Client, ClientExt};
pub use context::metadata::Metadata;
pub use context::timeout::Timeout;
pub use context::{get_context, get_server, try_get_context, try_get_server, Context};
pub use server::{Server, ServerConnection, ServerController, ServerHandle};
pub use trapeze_macros::*;
pub use types::protos::status::StatusExt;
pub use types::protos::{Code, Status};

#[doc(hidden)]
pub mod __codegen_prelude {
    pub use crate::client::request_handlers::RequestHandler;
    pub use crate::server::method_handlers::MethodHandler;
    pub use crate::service::{
        ClientStreamingMethod, DuplexStreamingMethod, ServerStreamingMethod, Service, UnaryMethod,
    };
}

#[doc(hidden)]
pub mod prelude {
    pub use std::future::Future;

    pub use futures::stream::Stream;

    pub use crate::Result;
}

pub mod stream {
    pub use async_stream::{stream, try_stream};
    pub use futures::stream::Stream;
}

pub mod codegen {
    pub use trapeze_codegen::*;
}
