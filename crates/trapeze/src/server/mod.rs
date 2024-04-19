use std::collections::HashMap;
use std::io::{ErrorKind, Result as IoResult};
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::pin;
use tokio::task::JoinSet;

use crate::context::timeout::Timeout;
use crate::context::{Context, WithContext};
use crate::io::MessageIo;
use crate::server::method_handlers::MethodHandler;
use crate::service::Service;
use crate::transport::{bind, Listener};
use crate::types::frame::StreamFrame;
use crate::types::protos::{Request, Status};

pub mod method_handlers;

#[derive(Default)]
pub struct Server {
    methods: HashMap<&'static str, Arc<dyn MethodHandler + Send + Sync>>,
    tasks: JoinSet<IoResult<()>>,
}

impl Server {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_service(&mut self, service: impl Service) -> &mut Self {
        let service = Arc::new(service);
        self.methods.extend(service.methods());
        self
    }

    pub async fn bind(&mut self, address: impl AsRef<str>) -> IoResult<()> {
        let mut listener = bind(address).await?;
        self.start(&mut listener).await
    }

    pub async fn start(&mut self, listener: &mut impl Listener) -> IoResult<()> {
        pin!(listener);
        loop {
            tokio::select! {
                Some(res) = self.tasks.join_next() => {
                    match res? {
                        Err(err) if err.kind() == ErrorKind::UnexpectedEof => {},
                        Ok(()) => {},
                        Err(err) => log::error!("Error handling client connection: {err}"),
                    }
                },
                conn = listener.accept() => {
                    let Ok(conn) = conn else {
                        continue;
                    };
                    let methods = self.methods.clone();
                    self.tasks.spawn(async move {
                        ServerConnection::new_with_methods(conn, methods)
                            .start()
                            .await
                    });
                },
                else => break,
            }
        }

        Ok(())
    }
}

pub struct ServerConnection {
    io: MessageIo,
    methods: HashMap<&'static str, Arc<dyn MethodHandler + Send + Sync>>,
    tasks: JoinSet<IoResult<()>>,
}

impl ServerConnection {
    pub fn new<C: AsyncRead + AsyncWrite + Send + 'static>(connection: C) -> ServerConnection {
        Self::new_with_services(connection, [])
    }

    pub fn new_with_services<C: AsyncRead + AsyncWrite + Send + 'static>(
        connection: C,
        services: impl IntoIterator<Item = Arc<dyn Service>>,
    ) -> ServerConnection {
        let mut methods = HashMap::default();
        for service in services {
            methods.extend(service.methods().into_iter());
        }

        Self::new_with_methods(connection, methods)
    }

    fn new_with_methods<C: AsyncRead + AsyncWrite + Send + 'static>(
        connection: C,
        methods: impl Into<HashMap<&'static str, Arc<dyn MethodHandler + Send + Sync>>>,
    ) -> ServerConnection {
        let mut tasks = JoinSet::<IoResult<()>>::new();
        let io = MessageIo::new(&mut tasks, connection);
        let methods = methods.into();

        ServerConnection { io, methods, tasks }
    }

    pub fn add_service(&mut self, service: impl Service) -> &mut Self {
        let service = Arc::new(service);
        self.methods.extend(service.methods());
        self
    }

    pub async fn start(&mut self) -> IoResult<()> {
        loop {
            tokio::select! {
                Some(res) = self.tasks.join_next() => {
                    res??;
                },
                Some((id, frame)) = self.io.rx.recv() => {
                    self.handle_message(id, &frame);
                },
                else => {
                    // no more messages to read, and no more taks to process
                    // we are done
                    break;
                },
            }
        }
        Ok(())
    }

    fn handle_message(&mut self, id: u32, frame: &StreamFrame) {
        let flags = frame.flags;

        let Some(mut stream) = self.io.stream(id) else {
            // The stream is not receiving any more messages.
            // This is probably a race condition between the stream finishing and
            // the cleanup of the stream forking.
            self.io.tx.send(id, Status::stream_in_use(id));
            return;
        };

        if (id % 2) != 1 {
            stream.tx.send(Status::invalid_stream_id(id));
            return;
        }

        let Ok(req) = frame.message.decode::<Request>() else {
            let ty = frame.message.ty;
            stream.tx.error(Status::expected_request(id, ty));
            return;
        };

        let Request {
            service,
            method,
            payload,
            timeout_nano,
            metadata,
        } = req;

        let ctx = Context {
            metadata: metadata.as_slice().into(),
            timeout: Timeout::from_nanos(timeout_nano),
        };

        let path = format!("/{service}/{method}");

        let Some(method) = self.methods.get(path.as_str()).cloned() else {
            stream.tx.error(Status::method_not_found(service, method));
            return;
        };

        self.tasks.spawn(
            async move {
                if let Err(status) = method.handle(flags, payload, &mut stream).await {
                    stream.tx.error(status);
                }
                Ok(())
            }
            .with_context(ctx),
        );
    }
}
