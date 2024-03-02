use std::future::Future;

use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};

use crate::encoded::{AsyncReadEncoded, Encoded, ReadEncodedError};

#[derive(Clone, Debug)]
pub enum Message {
    Request { id: u32, data: Encoded },
    Response { id: u32, data: Encoded },
}

#[derive(thiserror::Error, Debug)]
pub enum MessageReadError {
    #[error("Invalid message type `{0}`")]
    InvalidType(u8),
    #[error("Invalid message flag `{0}`")]
    InvalidFlags(u8),
    #[error("Invalid message id `{0}`")]
    InvalidId(u32),
    #[error("{0}")]
    EncodedError(#[from] ReadEncodedError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum MessageWriteError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

pub trait AsyncReadMessage {
    fn read_message(
        &mut self,
    ) -> impl Future<Output = std::result::Result<Message, MessageReadError>> + Send;
}

pub trait AsyncWriteMessage {
    fn write_message(
        &mut self,
        msg: &Message,
    ) -> impl Future<Output = std::result::Result<(), MessageWriteError>> + Send;
}

const REQUEST_TYPE: u8 = 1;
const RESPONSE_TYPE: u8 = 2;

impl<R: AsyncRead + Unpin + Send> AsyncReadMessage for R {
    async fn read_message(&mut self) -> std::result::Result<Message, MessageReadError> {
        let data_length = self.read_u32().await? as usize;
        let id = self.read_u32().await?;
        let ty = self.read_u8().await?;
        let flags = self.read_u8().await?;
        let data = self.read_encoded(data_length).await?;

        // Client initiated streams are odd.
        // Server initiated streams are even.
        // TTRPC does not yet supports server initiated streams.
        if id % 2 == 0 {
            return Err(MessageReadError::InvalidId(id));
        }

        // We only support unary messages (requests and response).
        // Data streams are not supported.
        if ty != REQUEST_TYPE && ty != RESPONSE_TYPE {
            return Err(MessageReadError::InvalidType(ty));
        }

        // Flags are currently only used with streams, which we don't support.
        if flags != 0 {
            return Err(MessageReadError::InvalidFlags(flags));
        };

        let msg = match ty {
            REQUEST_TYPE => Message::Request { id, data },
            RESPONSE_TYPE => Message::Response { id, data },
            _ => {
                return Err(MessageReadError::InvalidType(ty));
            }
        };

        Ok(msg)
    }
}

impl<W: AsyncWrite + Unpin + Send> AsyncWriteMessage for W {
    async fn write_message(&mut self, msg: &Message) -> std::result::Result<(), MessageWriteError> {
        let (ty, id, data) = match msg {
            Message::Request { id, data } => (1u8, id, data),
            Message::Response { id, data } => (2u8, id, data),
        };
        let mut buf = Vec::with_capacity(10 + data.len());
        buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
        buf.extend_from_slice(&id.to_be_bytes());
        buf.extend_from_slice(&ty.to_be_bytes());
        buf.extend_from_slice(&0u8.to_be_bytes());
        buf.extend_from_slice(data.as_slice());
        self.write_all(&buf).await?;
        Ok(())
    }
}
