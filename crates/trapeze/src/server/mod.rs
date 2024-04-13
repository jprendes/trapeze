use std::collections::HashMap;
use std::io::Result as IoResult;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task::JoinSet;

use crate::context::timeout::Timeout;
use crate::context::{Context, WithContext};
use crate::io::MessageIo;
use crate::service::Service;
use crate::types::frame::StreamFrame;
use crate::types::protos::{Request, Status};

pub mod method_handlers;

pub struct Server {
    io: MessageIo,
    services: HashMap<&'static str, Arc<dyn Service>>,
    tasks: JoinSet<IoResult<()>>,
}

impl Server {
    pub fn new<C: AsyncRead + AsyncWrite + Send + 'static>(connection: C) -> Server {
        let mut tasks = JoinSet::<IoResult<()>>::new();
        let io = MessageIo::new(&mut tasks, connection);
        let services = Default::default();

        Server {
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
                    self.handle_message(id, frame);
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

    fn handle_message(&mut self, id: u32, frame: StreamFrame) {
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
