use std::future::Future;
use std::io::Result as IoResult;
use std::sync::Arc;

use futures::FutureExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinSet;

use crate::context::metadata::Metadata;
use crate::context::timeout::Timeout;
use crate::context::Context;
use crate::io::{MessageIo, StreamIo};
use crate::transport::connect;
use crate::{Result, Status};

pub mod request_handlers;

type RequestFnBox = Box<dyn FnOnce(StreamIo, &mut JoinSet<IoResult<()>>) + Send>;

#[derive(Clone)]
pub struct Client {
    tx: UnboundedSender<RequestFnBox>,
    tasks: Arc<JoinSet<IoResult<()>>>,
    pub context: Context,
}

struct ClientInner {
    next_id: u32,
    io: MessageIo,
    tasks: JoinSet<IoResult<()>>,
}

impl Client {
    #[must_use]
    pub fn with_metadata(&self, metadata: impl Into<Metadata>) -> Self {
        Self {
            tx: self.tx.clone(),
            tasks: self.tasks.clone(),
            context: Context {
                timeout: self.context.timeout,
                metadata: metadata.into(),
            },
        }
    }

    #[must_use]
    pub fn with_timeout(&self, timeout: impl Into<Timeout>) -> Self {
        Self {
            tx: self.tx.clone(),
            tasks: self.tasks.clone(),
            context: Context {
                timeout: timeout.into(),
                metadata: self.context.metadata.clone(),
            },
        }
    }

    #[must_use]
    pub fn with_context(&self, context: impl Into<Context>) -> Self {
        Self {
            tx: self.tx.clone(),
            tasks: self.tasks.clone(),
            context: context.into(),
        }
    }
}

impl ClientInner {
    pub fn new<C: AsyncRead + AsyncWrite + Send + 'static>(connection: C) -> Self {
        let mut tasks = JoinSet::<IoResult<()>>::new();
        let io = MessageIo::new(&mut tasks, connection);
        let next_id = 1;

        Self { next_id, io, tasks }
    }

    pub async fn start(&mut self, mut req_rx: UnboundedReceiver<RequestFnBox>) -> IoResult<()> {
        loop {
            tokio::select! {
                Some(res) = self.tasks.join_next() => {
                    res??;
                },
                Some(fcn) = req_rx.recv() => {
                    let id = self.next_id;
                    self.next_id += 2;
                    let Some(stream) = self.io.stream(id) else {
                        log::error!("Ran out of stream ids");
                        continue;
                    };
                    fcn(stream, &mut self.tasks);
                },
                Some((id, _)) = self.io.rx.recv() => {
                    log::error!("Received a message with an invalid stream id `{id}`");
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
}

impl Client {
    pub fn new<C: AsyncRead + AsyncWrite + Send + 'static>(connection: C) -> Self {
        let (tx, rx) = unbounded_channel();
        let mut tasks = JoinSet::<IoResult<()>>::new();
        let context = Context::default();

        let mut inner = ClientInner::new(connection);
        tasks.spawn(async move { inner.start(rx).await });

        let tasks = Arc::new(tasks);

        Self { tx, tasks, context }
    }

    pub async fn connect(address: impl AsRef<str>) -> IoResult<Self> {
        let conn = connect(address).await?;
        Ok(Self::new(conn))
    }

    fn spawn_stream<Fut: Future<Output = Result<()>> + Send>(
        &self,
        f: impl FnOnce(StreamIo) -> Fut + Send + 'static,
    ) -> impl Future<Output = Result<()>> + Send {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(Box::new(move |stream, tasks| {
            tasks.spawn(async move {
                let _ = tx.send(f(stream).await);
                Ok(())
            });
        }));

        async move {
            let Ok(result) = rx.await else {
                return Err(Status::channel_closed());
            };
            result
        }
        .fuse()
    }
}
