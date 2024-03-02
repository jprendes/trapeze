trapeze_macros::include_proto!("ttrpc.proto");

pub use std::future::Future;

pub mod client;
mod constants;
pub mod context;
pub mod encoded;
pub mod error;
mod id_map;
mod message;
pub mod server;
pub mod service;
pub mod traits;

pub use trapeze_codegen as codegen;
pub use trapeze_macros::*;

pub use client::Client;
pub use error::*;
pub use server::{Server, ServerBuilder};

pub type Result<T, E = grpc::Status> = std::result::Result<T, E>;
