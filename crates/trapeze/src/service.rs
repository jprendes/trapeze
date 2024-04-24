use std::marker::PhantomData;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::server::method_handlers::MethodHandler;

pub trait Service: Send + Sync {
    fn methods(self: Arc<Self>) -> Vec<(&'static str, Arc<dyn MethodHandler + Send + Sync>)>;
}

pub struct UnaryMethod<Input, Output, Method> {
    pub(crate) method: Method,
    _phantom: PhantomData<Mutex<(Input, Output)>>,
}

pub struct ServerStreamingMethod<Input, Output, Method> {
    pub(crate) method: Method,
    _phantom: PhantomData<Mutex<(Input, Output)>>,
}

pub struct ClientStreamingMethod<Input, Output, Method> {
    pub(crate) method: Method,
    _phantom: PhantomData<Mutex<(Input, Output)>>,
}

pub struct DuplexStreamingMethod<Input, Output, Method> {
    pub(crate) method: Method,
    _phantom: PhantomData<Mutex<(Input, Output)>>,
}

impl<Input, Output, F> UnaryMethod<Input, Output, F> {
    pub fn new(method: F) -> Self {
        Self {
            method,
            _phantom: PhantomData,
        }
    }
}

impl<Input, Output, F> ServerStreamingMethod<Input, Output, F> {
    pub fn new(method: F) -> Self {
        Self {
            method,
            _phantom: PhantomData,
        }
    }
}

impl<Input, Output, F> ClientStreamingMethod<Input, Output, F> {
    pub fn new(method: F) -> Self {
        Self {
            method,
            _phantom: PhantomData,
        }
    }
}

impl<Input, Output, F> DuplexStreamingMethod<Input, Output, F> {
    pub fn new(method: F) -> Self {
        Self {
            method,
            _phantom: PhantomData,
        }
    }
}
