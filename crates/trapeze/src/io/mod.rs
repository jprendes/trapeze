use std::future::Future;
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};

//use flume::{unbounded_channel, UnboundedReceiver, SendError, UnboundedSender};
use prost::bytes::Bytes;
use thiserror::Error;
use tokio::io::{split, AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::pin;
use tokio::sync::mpsc::error::SendError as MpscSendError;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::task::JoinSet;

use crate::id_pool::{IdPool, IdPoolGuard};
use crate::types::encoding::{Decodeable as _, Encodeable, InvalidInput};
use crate::types::flags::Flags;
use crate::types::frame::{read_frame_bytes, Frame, StreamFrame};
use crate::types::message::Message;
use crate::types::protos::{Data, Response, Status};

#[derive(Clone)]
pub struct MessageSender {
    tx: UnboundedSender<(Bytes, oneshot::Sender<()>)>,
}

pub struct MessageReceiver {
    // Use an unbounded_channel sender to avoid overflowing the input buffer
    rx: UnboundedReceiver<Frame>,
    streams: IdPool<UnboundedSender<StreamFrame>>,
}

pub struct MessageIo {
    pub tx: MessageSender,
    pub rx: MessageReceiver,
}

#[derive(Error, Debug)]
pub enum SendError {
    #[error("Io error: {0}")]
    Io(#[from] IoError),

    #[error("Invalid input: {0}")]
    InvalidInput(#[from] InvalidInput),
}

impl SendError {
    pub fn channel_closed() -> Self {
        SendError::Io(IoError::new(IoErrorKind::BrokenPipe, "Channel closed"))
    }
}

pub struct SendResult(Result<oneshot::Receiver<()>, InvalidInput>);

impl Future for SendResult {
    type Output = Result<(), SendError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Self::Output> {
        match &mut self.0 {
            Err(err) => Poll::Ready(Err(err.clone().into())),
            Ok(receiver) => {
                pin!(receiver);
                match receiver.poll(cx) {
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(result) => {
                        Poll::Ready(result.map_err(|_| SendError::channel_closed()))
                    }
                }
            }
        }
    }
}

impl MessageSender {
    pub fn new(
        tasks: &mut JoinSet<IoResult<()>>,
        mut writer: impl AsyncWrite + Unpin + Send + 'static,
    ) -> Self {
        let (tx, mut rx) = unbounded_channel();
        let sender = Self { tx };
        tasks.spawn(async move {
            while let Some((mut bytes, ch)) = rx.recv().await {
                // Errors writing bytes to the stream interrupt the loop
                writer.write_all_buf(&mut bytes).await?;
                let _ = ch.send(());
            }
            Ok(())
        });
        sender
    }

    pub fn send<Msg: Message + Encodeable>(
        &self,
        id: u32,
        frame: impl Into<StreamFrame<Msg>>,
    ) -> SendResult {
        // Errors encoding the message do not interrupt the loop
        let rx = (move || {
            let frame = frame.into();
            let frame = frame.into_frame(id);
            let bytes = frame.encode_to_bytes()?;
            let (tx, rx) = oneshot::channel();
            let _ = self.tx.send((bytes, tx));
            Ok::<_, InvalidInput>(rx)
        })();

        SendResult(rx)
    }

    fn stream(&self, id: u32) -> StreamSender {
        let tx = self.clone();
        StreamSender { id, tx }
    }
}

impl MessageReceiver {
    pub fn new(
        tasks: &mut JoinSet<IoResult<()>>,
        mut reader: impl AsyncRead + Send + Unpin + 'static,
    ) -> Self {
        let (tx, rx) = unbounded_channel();
        let streams = IdPool::default();
        let receiver = Self { rx, streams };
        tasks.spawn(async move {
            loop {
                // Errors reading bytes from the stream interrupt the loop
                let bytes = read_frame_bytes(&mut reader).await?;

                // This is safe because RawFrame decode errors are delayed until the
                // message is accessed.
                // The only possible error is if `bytes` has less than `HEADER_LENGTH`
                // bytes, which is not possible here.
                let frame = Frame::decode(bytes).unwrap();

                let _ = tx.send(frame);
            }
        });
        receiver
    }

    pub async fn recv(&mut self) -> Option<(u32, StreamFrame)> {
        while let Some(frame) = self.rx.recv().await {
            let id = frame.id;
            let frame = frame.into_stream_frame();

            let Some(stream_tx) = self.streams.get(id) else {
                // there was no stream for this id, return the message
                return Some((id, frame));
            };

            // there was a stream for this id, so attempt to send it
            if let Err(MpscSendError(frame)) = stream_tx.send(frame) {
                // the stream was already closed, return the message and let consumers handle it
                return Some((id, frame));
            }
        }
        None
    }

    fn stream(&mut self, id: impl Into<Option<u32>>) -> Option<StreamReceiver> {
        let (tx, rx) = unbounded_channel();
        let Some(guard) = self.streams.claim(id, tx) else {
            return None;
        };
        let guard = Arc::new(guard);
        Some(StreamReceiver { rx, guard })
    }
}

impl MessageIo {
    pub fn new(
        tasks: &mut JoinSet<IoResult<()>>,
        connection: impl AsyncRead + AsyncWrite + Send + 'static,
    ) -> Self {
        let (reader, writer) = split(connection);

        let rx = MessageReceiver::new(tasks, reader);
        let tx = MessageSender::new(tasks, writer);

        Self { tx, rx }
    }

    pub fn stream(&mut self, id: impl Into<Option<u32>>) -> Option<StreamIo> {
        let rx = self.rx.stream(id)?;
        let tx = self.tx.stream(rx.id());
        Some(StreamIo { tx, rx })
    }
}

#[derive(Clone)]
pub struct StreamSender {
    id: u32,
    tx: MessageSender,
}

pub struct StreamReceiver {
    rx: UnboundedReceiver<StreamFrame>,
    guard: Arc<IdPoolGuard>,
}

pub struct StreamIo {
    pub tx: StreamSender,
    pub rx: StreamReceiver,
}

impl StreamSender {
    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn send<Msg: Message + Encodeable>(
        &self,
        frame: impl Into<StreamFrame<Msg>>,
    ) -> SendResult {
        self.tx.send(self.id, frame)
    }

    pub fn error(&self, status: Status) -> SendResult {
        self.send(Response::error(status))
    }

    pub fn respond<Payload: prost::Message + Default>(&self, payload: Payload) -> SendResult {
        self.send(Response::ok(payload))
    }

    pub fn data<Payload: prost::Message + Default>(&self, payload: Payload) -> SendResult {
        self.send(StreamFrame {
            flags: Flags::empty(),
            message: Data { payload },
        })
    }

    pub fn close_data(&self) -> SendResult {
        self.send(StreamFrame {
            flags: Flags::REMOTE_CLOSED | Flags::NO_DATA,
            message: Data { payload: () },
        })
    }
}

impl StreamReceiver {
    pub fn id(&self) -> u32 {
        self.guard.id()
    }

    pub fn guard(&self) -> Arc<IdPoolGuard> {
        self.guard.clone()
    }

    pub async fn recv(&mut self) -> Option<StreamFrame> {
        self.rx.recv().await
    }
}

impl StreamIo {
    pub fn id(&self) -> u32 {
        self.tx.id()
    }

    pub fn split(self) -> (StreamSender, StreamReceiver) {
        (self.tx, self.rx)
    }
}
