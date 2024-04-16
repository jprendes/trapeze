use std::fmt::Display;
use std::future::{ready, Ready};
use std::marker::PhantomData;

use futures::stream::{once, Once};

use crate::server::method_handlers::MethodHandler;
use crate::{Result, Status};

pub trait Service: Send + Sync {
    fn name(&self) -> &'static str;

    fn dispatch(&self, method: String) -> Box<dyn MethodHandler + Send + '_> {
        let service = self.name();
        Box::new(MethodNotFound { service, method })
    }
}

pub struct UnaryMethod<Input, Output, F> {
    pub(crate) f: F,
    _phantom: PhantomData<(Input, Output)>,
}

pub struct ServerStreamingMethod<Input, Output, F> {
    pub(crate) f: F,
    _phantom: PhantomData<(Input, Output)>,
}

pub struct ClientStreamingMethod<Input, Output, F> {
    pub(crate) f: F,
    _phantom: PhantomData<(Input, Output)>,
}

pub struct DuplexStreamingMethod<Input, Output, F> {
    pub(crate) f: F,
    _phantom: PhantomData<(Input, Output)>,
}

pub struct MethodNotFound<S: Display + Send, M: Display + Send> {
    pub service: S,
    pub method: M,
}

impl<Input, Output, F> UnaryMethod<Input, Output, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: PhantomData,
        }
    }
}

impl<Input, Output, F> ServerStreamingMethod<Input, Output, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: PhantomData,
        }
    }
}

impl<Input, Output, F> ClientStreamingMethod<Input, Output, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: PhantomData,
        }
    }
}

impl<Input, Output, F> DuplexStreamingMethod<Input, Output, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: PhantomData,
        }
    }
}

impl<S: Display + Send, M: Display + Send> MethodNotFound<S, M> {
    pub fn into_future<T>(self) -> Ready<Result<T>> {
        ready(Err(Status::method_not_found(&self.service, &self.method)))
    }

    pub fn into_stream<T>(self) -> Once<Ready<Result<T>>> {
        once(self.into_future())
    }
}
