use std::collections::HashMap;
use std::sync::Arc;

use tokio::io::{split, ReadHalf, WriteHalf};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::constants::{REQUEST_TIMEOUT_STATUS, RESPONSE_TOO_LONG_ENCODED};
use crate::context::{Context, Timeout, CONTEXT};
use crate::encoded::{DecodeError, EncodeError, Encoded};
use crate::error::{Code, Status};
use crate::grpc::{Request, Response};
use crate::id_map::IdMap;
use crate::message::{
    AsyncReadMessage, AsyncWriteMessage, Message, MessageReadError, MessageWriteError,
};
use crate::service::Service;
use crate::traits::{AsyncRead, AsyncWrite};

pub struct Server<Rx: AsyncRead, Tx: AsyncWrite> {
    inner: Arc<ServerInner<Rx, Tx>>,
}

pub struct ServerInner<Rx: AsyncRead, Tx: AsyncWrite> {
    rx: RwLock<Rx>,
    tx: RwLock<Tx>,
    services: HashMap<String, Arc<dyn Service>>,
    streams: IdMap<()>,
}

impl<Rx: AsyncRead, Tx: AsyncWrite> Clone for Server<Rx, Tx> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum RequestHandlingError {
    #[error("Message ID `{0}` is already in use")]
    IdInUse(u32),
    #[error("{0}")]
    Encode(#[from] EncodeError),
    #[error("{0}")]
    Decode(#[from] DecodeError),
    #[error("{0}")]
    MessageWrite(#[from] MessageWriteError),
}

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Server received a response")]
    ReceivedResponse,
    #[error("{0}")]
    MessageRead(#[from] MessageReadError),
    #[error("{0}")]
    RequestHandling(#[from] RequestHandlingError),
}

#[derive(Default)]
pub struct ServerBuilder {
    services: Vec<Arc<dyn Service>>,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_service(mut self, service: impl Service + 'static) -> Self {
        self.services.push(Arc::new(service) as Arc<dyn Service>);
        self
    }

    pub fn build<C: AsyncRead + AsyncWrite>(
        self,
        connection: C,
    ) -> Server<ReadHalf<C>, WriteHalf<C>> {
        Server::new(connection, self.services)
    }
}

impl<Rx: AsyncRead, Tx: AsyncWrite> Server<Rx, Tx> {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}

impl<C: AsyncRead + AsyncWrite> Server<ReadHalf<C>, WriteHalf<C>> {
    pub fn new(connection: C, services: impl IntoIterator<Item = Arc<dyn Service>>) -> Self {
        let (rx, tx) = split(connection);
        let services = services
            .into_iter()
            .map(|service| (service.name().to_string(), service))
            .collect();
        Self {
            inner: Arc::new(ServerInner {
                tx: RwLock::new(tx),
                rx: RwLock::new(rx),
                services,
                streams: Default::default(),
            }),
        }
    }
}

impl<Rx: AsyncRead, Tx: AsyncWrite> Server<Rx, Tx> {
    pub fn start(&self) -> JoinHandle<Result<(), ServerError>> {
        self.inner.start()
    }
}

impl<Rx: AsyncRead, Tx: AsyncWrite> ServerInner<Rx, Tx> {
    pub fn start(self: &Arc<Self>) -> JoinHandle<Result<(), ServerError>> {
        let server = self.clone();
        tokio::spawn(async move { server.run().await })
    }

    async fn run(self: &Arc<Self>) -> Result<(), ServerError> {
        loop {
            match self.rx.write().await.read_message().await? {
                Message::Response { .. } => {
                    return Err(ServerError::ReceivedResponse);
                }
                Message::Request { id, data } => {
                    self.handle_request(id, data).await?;
                }
            }
        }
    }

    async fn handle_request(
        self: &Arc<Self>,
        id: u32,
        req: Encoded,
    ) -> Result<(), RequestHandlingError> {
        let Some(guard) = self.streams.claim(id, ()).await else {
            return Err(RequestHandlingError::IdInUse(id));
        };
        let req = req.decode()?;

        let this = self.clone();

        tokio::spawn(async move {
            // move the id guard to the async task
            let _guard = guard;

            let res = this.dispatch_request(req).await;
            let res = Encoded::encode(&res).unwrap_or_else(|_| RESPONSE_TOO_LONG_ENCODED.clone());

            this.tx
                .write()
                .await
                .write_message(&Message::Response { id, data: res })
                .await?;

            Ok::<(), RequestHandlingError>(())
        });

        Ok(())
    }

    async fn dispatch_request(self: &Arc<Self>, req: Request) -> Response {
        let Request {
            service,
            method,
            payload,
            metadata,
            timeout_nano,
        } = req;

        let context = Arc::new(Context {
            metadata: metadata.as_slice().into(),
            timeout: Timeout::from_nanos(timeout_nano),
        });

        let Some(service) = self.services.get(&service).cloned() else {
            let status = Status::new(
                Code::NotFound,
                format!("/{service}/{method} is not supported"),
            );

            return Response {
                status: Some(status),
                payload: vec![],
            };
        };

        // Safe because payload comes from a Request, which was itself encoded
        let payload = Encoded::buffer(payload).unwrap();

        let response = CONTEXT
            .scope(context.clone(), async move {
                if context.timeout.is_zero() {
                    service.dispatch(&method, payload).await
                } else {
                    tokio::time::timeout(*context.timeout, async move {
                        service.dispatch(&method, payload).await
                    })
                    .await
                    .unwrap_or_else(|_| Err(REQUEST_TIMEOUT_STATUS.clone()))
                }
            })
            .await;

        match response {
            Ok(res) => Response {
                status: Some(Status::new(Code::Ok, "")),
                payload: res.into_inner(),
            },
            Err(status) => Response {
                status: Some(status),
                payload: vec![],
            },
        }
    }
}
