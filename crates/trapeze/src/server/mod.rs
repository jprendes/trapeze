use std::collections::HashMap;
use std::io::{ErrorKind, Result as IoResult};
use std::sync::Arc;

use futures::{pin_mut, FutureExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::JoinSet;

use crate::context::timeout::Timeout;
use crate::context::{Context, WithContext};
use crate::io::MessageIo;
use crate::server::method_handlers::MethodHandler;
use crate::service::Service;
use crate::transport::{bind, Listener};
use crate::types::frame::StreamFrame;
use crate::types::protos::{Request, Status};

pub mod controller;
pub mod handle;
pub mod method_handlers;

pub use controller::ServerController;
pub use handle::ServerHandle;

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

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn register(mut self, service: impl Service) -> Self {
        self.methods.extend(service.methods());
        self
    }

    pub async fn bind(self, address: impl AsRef<str>) -> IoResult<ServerHandle> {
        let listener = bind(address).await?;
        Ok(self.start(listener))
    }

    pub fn start(mut self, mut listener: impl Listener) -> ServerHandle {
        ServerHandle::spawn(move |controller, shutdown| async move {
            let shutdown = shutdown.notified().fuse();
            pin_mut!(shutdown);
            loop {
                tokio::select! {
                    conn = listener.accept() => {
                        let Ok(conn) = conn else {
                            continue;
                        };
                        let methods = self.methods.clone();
                        let controller = controller.clone();
                        self.tasks.spawn(async move {
                            ServerConnection::new_with_methods(conn, methods)
                                .with_controller(controller)
                                .start()
                                .await
                        });
                    },
                    Some(res) = self.tasks.join_next() => {
                        handle_task_result(res?);
                    },
                    () = &mut shutdown => break,
                    else => break,
                }
            }

            drop(listener);

            // drain any remaining tasks after a shutdown
            while let Some(res) = self.tasks.join_next().await {
                handle_task_result(res?);
            }

            Ok(())
        })
    }
}

fn handle_task_result(result: IoResult<()>) {
    match result {
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => {}
        Ok(()) => {}
        Err(err) => log::error!("Error handling client connection: {err}"),
    }
}

pub struct ServerConnection {
    io: MessageIo,
    methods: HashMap<&'static str, Arc<dyn MethodHandler + Send + Sync>>,
    tasks: JoinSet<IoResult<()>>,
    controller: Option<ServerController>,
}

impl ServerConnection {
    pub fn new<C: AsyncRead + AsyncWrite + Send + 'static>(connection: C) -> ServerConnection {
        Self::new_with(connection, [])
    }

    pub fn new_with<'a, C: AsyncRead + AsyncWrite + Send + 'static>(
        connection: C,
        services: impl IntoIterator<Item = &'a dyn Service>,
    ) -> ServerConnection {
        let mut methods = HashMap::default();
        for service in services {
            methods.extend(service.methods().into_iter());
        }

        Self::new_with_methods(connection, methods)
    }

    fn with_controller(&mut self, controller: ServerController) -> &mut Self {
        self.controller = Some(controller);
        self
    }

    fn new_with_methods<C: AsyncRead + AsyncWrite + Send + 'static>(
        connection: C,
        methods: impl Into<HashMap<&'static str, Arc<dyn MethodHandler + Send + Sync>>>,
    ) -> ServerConnection {
        let mut tasks = JoinSet::<IoResult<()>>::new();
        let io = MessageIo::new(&mut tasks, connection);
        let methods = methods.into();
        let controller = None;

        ServerConnection {
            io,
            methods,
            tasks,
            controller,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub fn register(&mut self, service: impl Service) -> &mut Self {
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
            .with_context(ctx, self.controller.clone()),
        );
    }
}
