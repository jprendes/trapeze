
pub mod __service_module_name__ {
    pub struct Service<T: super::__service_name__>(pub T);
    
    impl<T: super::__service_name__> std::ops::Deref for Service<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: super::__service_name__> trapeze::__codegen_prelude::Service for Service<T> {
        fn name(&self) -> &'static str {
            "__service_package__.__service_proto_name__"
        }

        fn dispatch(
            &self,
            method: std::string::String,
        ) -> std::boxed::Box<dyn trapeze::__codegen_prelude::MethodHandler + Send + '_>
        {
            match method.as_str() {
                __dispatch_branches__
                _ => {
                    let service = self.name();
                    std::boxed::Box::new(trapeze::__codegen_prelude::MethodNotFound { service, method })
                },
            }
        }
    }

    impl<T: super::__service_name__ + Clone> Clone for Service<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl<T: super::__service_name__ + Default> Default for Service<T> {
        fn default() -> Self {
            Self(Default::default())
        }
    }
}

__service_comments__
pub trait __service_name__: Send + Sync + 'static {
    __trait_methods__

    fn service() -> __service_module_name__::Service<Self> where Self: Sized + Default {
        __service_module_name__::Service(Default::default())
    }
}

//pub fn __service_name__<T: __service_name__>(service: T) -> __service_module_name__::Service<T> {
//    __service_module_name__::Service(service)
//}

impl __service_name__ for trapeze::Client {
    __client_methods__
}
