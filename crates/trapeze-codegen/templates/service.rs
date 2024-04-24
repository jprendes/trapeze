#[allow(dead_code, non_snake_case)]
pub fn __service_name__<T: __service_name__>(target: impl std::convert::Into<std::sync::Arc<T>>) -> impl trapeze::__codegen_prelude::Service {
    struct Service<T: __service_name__> {
        target: std::sync::Arc<T>
    }
    impl<T: __service_name__> trapeze::__codegen_prelude::Service for Service<T> {
        fn methods(&self) -> std::vec::Vec<(&'static str, std::sync::Arc<dyn trapeze::__codegen_prelude::MethodHandler + Send + Sync>)> {
            let target = &self.target;
            vec![
                __dispatch_branches__
            ]
        }
    }
    Service { target: target.into() }
}

__service_comments__
pub trait __service_name__: Send + Sync + 'static {
    __trait_methods__
}

impl __service_name__ for trapeze::Client {
    __client_methods__
}
