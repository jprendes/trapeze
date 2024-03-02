
pub mod __service_module_name__ {
    pub struct Service<T: super::__service_name__>(pub T);
    
    impl<T: super::__service_name__> std::ops::Deref for Service<T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl<T: super::__service_name__> trapeze::service::Service for Service<T> {
        fn name(&self) -> &'static str {
            "__service_package__.__service_proto_name__"
        }

        fn dispatch<'a, 'b>(
            &'a self,
            method: &'b str,
            payload: trapeze::encoded::Encoded,
        ) -> std::pin::Pin<std::boxed::Box<dyn std::future::Future<Output = trapeze::Result<trapeze::encoded::Encoded>> + Send + 'a>>
        where
            Self: Sync + 'a,
            'b: 'a
        {
            std::boxed::Box::pin(async move {
                match method {
                    __dispatch_branches__
                    _ => {
                        let code = trapeze::Code::NotFound;
                        let message = format!("/__service_package__.__service_proto_name__/{method} is not supported");
                        let response = trapeze::Status::new(code, message);
                        Ok(trapeze::encoded::Encoded::encode(&response)?)
                    },
                }
            })
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

pub trait __service_name__: Send + Sync + 'static {
    __trait_methods__

    fn service() -> __service_module_name__::Service<Self> where Self: Sized + Default {
        __service_module_name__::Service(Default::default())
    }
}

impl<Rx: trapeze::traits::AsyncRead, Tx: trapeze::traits::AsyncWrite> __service_name__ for trapeze::client::Client<Rx, Tx> {
    __client_methods__
}
