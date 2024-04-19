use std::future::Future;

use async_trait::async_trait;
use futures::future::pending;
use futures::{Stream, TryStreamExt as _};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::context::get_context;
use crate::context::timeout::Timeout;
use crate::io::{StreamIo, StreamReceiver, StreamSender};
use crate::service::{
    ClientStreamingMethod, DuplexStreamingMethod, ServerStreamingMethod, UnaryMethod,
};
use crate::types::encoding::BufExt;
use crate::types::flags::Flags;
use crate::types::protos::raw_bytes::RawBytes;
use crate::types::protos::{Data, Status};
use crate::Result;

#[async_trait]
pub trait MethodHandler {
    async fn handle(&self, flags: Flags, payload: RawBytes, stream: &mut StreamIo) -> Result<()>;
}

macro_rules! try_join_all {
    ($($e:expr),* $(,)?) => { async {
        tokio::try_join! { $($e),* }.map(|_| ())
    } };
}

macro_rules! join_first {
    ($($e:expr),* $(,)?) => { tokio::select! {
        $(res = $e => res),+
    } };
}

#[async_trait]
impl<
        Input: prost::Message + Default,
        Output: prost::Message + Default,
        FutOut: Future<Output = Result<Output>> + Send,
        F: Fn(Input) -> FutOut + Send + Sync,
    > MethodHandler for UnaryMethod<Input, Output, F>
{
    async fn handle(&self, flags: Flags, payload: RawBytes, stream: &mut StreamIo) -> Result<()> {
        if !flags.is_empty() {
            // Unary methos should have empty flags
            return Err(Status::invalid_request_flags(Flags::empty(), flags));
        }

        let rx = RwLock::new(&mut stream.rx);

        let payload: Input = payload.decode()?;

        let fut = (self.method)(payload);

        let output = handle_server_unary(&stream.tx, fut);
        let monitor = monitor_client_stream(&rx);
        let timeout = handle_timeout();

        join_first! {
            try_join_all! {
                output,
            },
            monitor,
            timeout,
        }
    }
}

#[async_trait]
impl<
        Input: prost::Message + Default,
        Output: prost::Message + Default,
        StrmOut: Stream<Item = Result<Output>> + Send,
        F: Fn(Input) -> StrmOut + Send + Sync,
    > MethodHandler for ServerStreamingMethod<Input, Output, F>
{
    async fn handle(&self, flags: Flags, payload: RawBytes, stream: &mut StreamIo) -> Result<()> {
        let rx = RwLock::new(&mut stream.rx);

        if flags.bits() != Flags::REMOTE_CLOSED.bits() {
            // REMOTE_CLOSED must be set (as the client is not a stream)
            // NO_DATA must not be set, as we need to parse a payload
            return Err(Status::invalid_request_flags(Flags::REMOTE_CLOSED, flags));
        }

        let payload: Input = payload.decode()?;

        let output_strm = (self.method)(payload);

        let output = handle_server_stream(&stream.tx, output_strm);
        let monitor = monitor_client_stream(&rx);
        let timeout = handle_timeout();

        join_first! {
            try_join_all! {
                output,
            },
            monitor,
            timeout,
        }
    }
}

#[async_trait]
impl<
        Input: prost::Message + Default,
        Output: prost::Message + Default,
        FutOut: Future<Output = Result<Output>> + Send,
        F: Fn(UnboundedReceiverStream<Input>) -> FutOut + Send + Sync,
    > MethodHandler for ClientStreamingMethod<Input, Output, F>
{
    async fn handle(&self, flags: Flags, payload: RawBytes, stream: &mut StreamIo) -> Result<()> {
        let rx = RwLock::new(&mut stream.rx);

        if flags.bits() != (Flags::REMOTE_OPEN | Flags::NO_DATA).bits() {
            // REMOTE_OPEN must be set (as the client is a stream)
            // NO_DATA must be set, as the request doesn't include a stream payload
            return Err(Status::invalid_request_flags(
                Flags::REMOTE_OPEN | Flags::NO_DATA,
                flags,
            ));
        }

        let () = payload.decode()?;

        let (input_tx, input_strm) = make_input_stream();

        let output_fut = (self.method)(input_strm);

        let output = handle_server_unary(&stream.tx, output_fut);
        let input = handle_client_stream(&rx, input_tx);
        let monitor = monitor_client_stream(&rx);
        let timeout = handle_timeout();

        join_first! {
            try_join_all! {
                input,
                output,
            },
            monitor,
            timeout,
        }
    }
}

#[async_trait]
impl<
        Input: prost::Message + Default,
        Output: prost::Message + Default,
        StrmOut: Stream<Item = Result<Output>> + Send,
        F: Fn(UnboundedReceiverStream<Input>) -> StrmOut + Send + Sync,
    > MethodHandler for DuplexStreamingMethod<Input, Output, F>
{
    async fn handle(&self, flags: Flags, payload: RawBytes, stream: &mut StreamIo) -> Result<()> {
        let rx = RwLock::new(&mut stream.rx);

        if flags.bits() != (Flags::REMOTE_OPEN | Flags::NO_DATA).bits() {
            // REMOTE_OPEN must be set (as the client is a stream)
            // NO_DATA must be set, as the request doesn't include a stream payload
            return Err(Status::invalid_request_flags(
                Flags::REMOTE_OPEN | Flags::NO_DATA,
                flags,
            ));
        }

        let () = payload.decode()?;

        let (input_tx, input_strm) = make_input_stream();

        let output_strm = (self.method)(input_strm);

        let output = handle_server_stream(&stream.tx, output_strm);
        let input = handle_client_stream(&rx, input_tx);
        let monitor = monitor_client_stream(&rx);
        let timeout = handle_timeout();

        join_first! {
            try_join_all! {
                input,
                output,
            },
            monitor,
            timeout,
        }
    }
}

fn make_input_stream<Input>() -> (UnboundedSender<Input>, UnboundedReceiverStream<Input>) {
    let (tx, rx) = unbounded_channel::<Input>();
    let strm = UnboundedReceiverStream::new(rx);
    (tx, strm)
}

fn handle_client_stream<'a, Input: prost::Message + Default + 'a>(
    rx: &'a RwLock<&'a mut StreamReceiver>,
    tx: UnboundedSender<Input>,
) -> impl Future<Output = Result<()>> + Send + '_ {
    // lock the mutex synchronously to avoid other handlers getting a lock before us
    let mut rx = rx.try_write().unwrap();
    async move {
        while let Some(frame) = rx.recv().await {
            let Data { payload } = frame.message.decode::<Data>()?;

            if frame.flags.contains(Flags::NO_DATA) {
                payload.ensure_empty()?;
            } else {
                let _ = tx.send(payload.decode()?);
            }

            if frame.flags.contains(Flags::REMOTE_CLOSED) {
                break;
            }
        }

        Ok(())
    }
}

async fn monitor_client_stream(rx: &RwLock<&mut StreamReceiver>) -> Result<()> {
    let mut rx = rx.write().await;
    if rx.recv().await.is_some() {
        return Err(Status::stream_closed(rx.id()));
    }
    Ok(())
}

async fn handle_server_stream<Output: prost::Message + Default>(
    tx: &StreamSender,
    strm: impl Stream<Item = Result<Output>>,
) -> Result<()> {
    tokio::pin!(strm);

    while let Some(data) = strm.try_next().await? {
        tx.data(data).await?;
    }

    tx.close_data().await?;

    Ok(())
}

async fn handle_server_unary<Output: prost::Message + Default>(
    tx: &StreamSender,
    fut: impl Future<Output = Result<Output>>,
) -> Result<()> {
    let response = fut.await?;
    tx.respond(response).await?;
    Ok(())
}

async fn handle_timeout() -> Result<()> {
    let t = get_context().timeout;
    match t {
        Timeout::Duration(t) => sleep(t).await,
        Timeout::None => pending::<()>().await,
    }
    Err(Status::timeout())
}
