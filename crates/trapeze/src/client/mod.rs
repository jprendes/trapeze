use std::future::Future;
use std::io::Result as IoResult;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use futures::FutureExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinSet;

use crate::context::metadata::Metadata;
use crate::context::timeout::Timeout;
use crate::context::Context;
use crate::io::{MessageIo, SendResult, StreamIo};
use crate::transport::connect;
use crate::types::encoding::Encodeable;
use crate::types::frame::StreamFrame;
use crate::types::message::Message;
use crate::{Result, Status};

pub mod request_handlers;

type RequestFnBox = Box<dyn FnOnce(StreamIo, &mut JoinSet<IoResult<()>>) + Send>;

#[derive(Clone)]
pub struct Client {
    tx: UnboundedSender<RequestFnBox>,
    _tasks: Arc<JoinSet<IoResult<()>>>,
    context: Context,
}

struct ClientInner {
    next_id: u32,
    io: MessageIo,
    tasks: JoinSet<IoResult<()>>,
}

impl Deref for Client {
    type Target = Context;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
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

        Self {
            tx,
            _tasks: tasks,
            context,
        }
    }

    pub async fn connect(address: impl AsRef<str>) -> IoResult<Self> {
        let conn = connect(address).await?;
        Ok(Self::new(conn))
    }

    fn spawn_stream<Fut: Future<Output = Result<()>> + Send, Msg: Message + Encodeable>(
        &self,
        frame: impl Into<StreamFrame<Msg>> + Send + 'static,
        f: impl FnOnce(SendResult, StreamIo) -> Fut + Send + 'static,
    ) -> impl Future<Output = Result<()>> + Send {
        let (tx, rx) = oneshot::channel();
        let _ = self.tx.send(Box::new(move |stream, tasks| {
            let res = stream.tx.send(frame);
            tasks.spawn(async move {
                let _ = tx.send(f(res, stream).await);
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

pub trait ClientExt: Clone + Deref<Target = Context> + DerefMut {
    #[must_use]
    fn with_metadata(&self, metadata: impl Into<Metadata>) -> Self {
        let mut this = self.clone();
        this.metadata = metadata.into();
        this
    }

    #[must_use]
    fn with_timeout(&self, timeout: impl Into<Timeout>) -> Self {
        let mut this = self.clone();
        this.timeout = timeout.into();
        this
    }

    #[must_use]
    fn with_context(&self, context: impl Into<Context>) -> Self {
        let mut this = self.clone();
        *this = context.into();
        this
    }
}

impl<T: Clone + Deref<Target = Context> + DerefMut> ClientExt for T {}

impl AsRef<Client> for Client {
    fn as_ref(&self) -> &Client {
        self
    }
}
