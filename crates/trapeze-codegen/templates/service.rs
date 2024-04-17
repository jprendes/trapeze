#[allow(dead_code, non_snake_case)]
pub fn __service_name__<T: __service_name__>() -> (&'static str, fn(&T, std::string::String) -> std::boxed::Box<dyn trapeze::__codegen_prelude::MethodHandler + Send + '_>) {
    fn dispatch<T: __service_name__>(
        target: &T,
        method: std::string::String,
    ) -> std::boxed::Box<dyn trapeze::__codegen_prelude::MethodHandler + Send + '_> {
        match method.as_str() {
            __dispatch_branches__
            _ => {
                let service = "__service_package__.__service_proto_name__";
                std::boxed::Box::new(trapeze::__codegen_prelude::MethodNotFound { service, method })
            },
        }
    }

    ("__service_package__.__service_proto_name__", dispatch)
}

__service_comments__
pub trait __service_name__: Send + Sync + 'static {
    __trait_methods__
}

impl __service_name__ for trapeze::Client {
    __client_methods__
}
