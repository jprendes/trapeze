use std::sync::Arc;

use tokio::io::{split, ReadHalf, WriteHalf};
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::constants::REQUEST_TIMEOUT_ENCODED;
use crate::context::{Context, Metadata, Timeout};
use crate::encoded::Encoded;
use crate::error::Status;
use crate::grpc::{Request, Response};
use crate::id_map::IdMap;
use crate::message::{AsyncReadMessage as _, AsyncWriteMessage, Message, MessageReadError};
use crate::traits::{AsyncRead, AsyncWrite};
use crate::Code;
use crate::Result;

pub struct Client<Rx: AsyncRead, Tx: AsyncWrite> {
    inner: Arc<ClientInner<Rx, Tx>>,
    pub context: Context,
}

pub struct ClientInner<Rx: AsyncRead, Tx: AsyncWrite> {
    rx: RwLock<Rx>,
    tx: RwLock<Tx>,
    streams: IdMap<Sender<Encoded>>,
}

impl<Rx: AsyncRead, Tx: AsyncWrite> Clone for Client<Rx, Tx> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            context: self.context.clone(),
        }
    }
}

impl<Rx: AsyncRead, Tx: AsyncWrite> Client<Rx, Tx> {
    pub fn with_metadata(&self, metadata: impl Into<Metadata>) -> Self {
        Self {
            inner: self.inner.clone(),
            context: Context {
                timeout: self.context.timeout,
                metadata: metadata.into(),
            },
        }
    }

    pub fn with_timeout(&self, timeout: impl Into<Timeout>) -> Self {
        Self {
            inner: self.inner.clone(),
            context: Context {
                timeout: timeout.into(),
                metadata: self.context.metadata.clone(),
            },
        }
    }

    pub fn with_context(&self, context: impl Into<Context>) -> Self {
        Self {
            inner: self.inner.clone(),
            context: context.into(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Client received a request")]
    ReceivedRequest,
    #[error("{0}")]
    MessageRead(#[from] MessageReadError),
}

impl<C: AsyncRead + AsyncWrite> Client<ReadHalf<C>, WriteHalf<C>> {
    pub fn new(connection: C) -> Self {
        let (rx, tx) = split(connection);
        Self {
            inner: Arc::new(ClientInner {
                tx: RwLock::new(tx),
                rx: RwLock::new(rx),
                streams: Default::default(),
            }),
            context: Default::default(),
        }
    }
}

impl<Rx: AsyncRead, Tx: AsyncWrite> Client<Rx, Tx> {
    pub fn start(&self) -> JoinHandle<Result<(), ClientError>> {
        self.inner.start()
    }
}

impl<Rx: AsyncRead, Tx: AsyncWrite> ClientInner<Rx, Tx> {
    pub fn start(self: &Arc<Self>) -> JoinHandle<Result<(), ClientError>> {
        let client = self.clone();
        tokio::spawn(async move { client.run().await })
    }

    async fn run(self: &Arc<Self>) -> std::result::Result<(), ClientError> {
        loop {
            match self.rx.write().await.read_message().await? {
                Message::Request { .. } => {
                    return Err(ClientError::ReceivedRequest);
                }
                Message::Response { id, data } => {
                    self.handle_response(id, data).await;
                }
            }
        }
    }

    async fn handle_response(self: &Arc<Self>, id: u32, data: Encoded) {
        if let Some(sender) = self.streams.borrow(id).await {
            let _ = sender.send(data).await;
        }
    }

    async fn request<T: prost::Message + Default>(
        self: &Arc<Self>,
        service: &str,
        method: &str,
        context: &Context,
        payload: &impl prost::Message,
    ) -> Result<T> {
        let payload = Encoded::encode(payload)?;

        let (sender, mut receiver) = channel(1);

        let guard = self.streams.claim_any(sender).await;
        let id = guard.id();

        let metadata = context.metadata.clone().into();
        let timeout = context.timeout;

        let data = Request {
            service: service.to_string(),
            method: method.to_string(),
            payload: payload.into_inner(),
            timeout_nano: context.timeout.as_nanos(),
            metadata,
        };

        let data = Encoded::encode(&data)?;

        self.tx
            .write()
            .await
            .write_message(&Message::Request { id, data })
            .await?;

        let response = if timeout.is_zero() {
            receiver.recv().await
        } else {
            tokio::time::timeout(*timeout, async move { receiver.recv().await })
                .await
                .unwrap_or_else(|_| Some(REQUEST_TIMEOUT_ENCODED.clone()))
        };

        let res: Response = response
            .ok_or_else(|| Status {
                code: Code::Cancelled.into(),
                message: "Channel closed".to_string(),
                ..Default::default()
            })?
            .decode()?;

        if let Some(status) = res.status {
            if status.code() != Code::Ok {
                return Err(status);
            }
        }

        let Response { payload, .. } = res;

        Ok(Encoded::buffer(payload)?.decode()?)
    }
}

pub trait ClientImpl {
    fn request<T: prost::Message + Default>(
        &self,
        service: &str,
        method: &str,
        data: &impl prost::Message,
    ) -> impl std::future::Future<Output = Result<T>> + Send;
}

impl<Rx: AsyncRead, Tx: AsyncWrite> ClientImpl for Client<Rx, Tx> {
    async fn request<T: prost::Message + Default>(
        &self,
        service: &str,
        method: &str,
        payload: &impl prost::Message,
    ) -> Result<T> {
        self.inner
            .request(service, method, &self.context, payload)
            .await
    }
}
