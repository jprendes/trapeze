use std::collections::HashMap;
use std::io::{ErrorKind, Result as IoResult};
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::pin;
use tokio::task::JoinSet;

use crate::context::timeout::Timeout;
use crate::context::{Context, WithContext};
use crate::io::MessageIo;
use crate::service::Service;
use crate::transport::{bind, Listener};
use crate::types::frame::StreamFrame;
use crate::types::protos::{Request, Status};

pub mod method_handlers;

#[derive(Default)]
pub struct Server {
    services: HashMap<&'static str, Arc<dyn Service>>,
    tasks: JoinSet<IoResult<()>>,
}

impl Server {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_service(&mut self, service: impl Service + 'static) -> &mut Self {
        self.services.insert(service.name(), Arc::new(service));
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
                    let services = self.services.clone();
                    self.tasks.spawn(async move {
                        ServerConnection::new_with_services(conn, services)
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
    services: HashMap<&'static str, Arc<dyn Service>>,
    tasks: JoinSet<IoResult<()>>,
}

impl ServerConnection {
    pub fn new<C: AsyncRead + AsyncWrite + Send + 'static>(connection: C) -> ServerConnection {
        Self::new_with_services(connection, HashMap::default())
    }

    pub fn new_with_services<C: AsyncRead + AsyncWrite + Send + 'static>(
        connection: C,
        services: HashMap<&'static str, Arc<dyn Service>>,
    ) -> ServerConnection {
        let mut tasks = JoinSet::<IoResult<()>>::new();
        let io = MessageIo::new(&mut tasks, connection);

        ServerConnection {
            io,
            services,
            tasks,
        }
    }

    pub fn add_service(&mut self, service: impl Service + 'static) -> &mut Self {
        self.services.insert(service.name(), Arc::new(service));
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

        let Some(service) = self.services.get(service.as_str()).cloned() else {
            stream.tx.error(Status::method_not_found(service, method));
            return;
        };

        self.tasks.spawn(
            async move {
                let method = service.dispatch(method);
                if let Err(status) = method.handle(flags, payload, &mut stream).await {
                    stream.tx.error(status);
                }
                Ok(())
            }
            .with_context(ctx),
        );
    }
}
