#[allow(dead_code, non_snake_case)]
pub fn __service_name__<T: __service_name__>(target: std::sync::Arc<T>) -> std::vec::Vec<(&'static str, std::sync::Arc<dyn trapeze::__codegen_prelude::MethodHandler + Send + Sync>)> {
    vec![
        __dispatch_branches__
    ]
}

__service_comments__
pub trait __service_name__: trapeze::__codegen_prelude::Sealed + Send + Sync + 'static {
    __trait_methods__
}

impl __service_name__ for trapeze::Client {
    __client_methods__
}
