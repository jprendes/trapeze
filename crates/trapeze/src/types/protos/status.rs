use std::fmt::Display;

use prost::EncodeError;
pub use prost_types::Any;
use thiserror::Error;

pub use super::Code;
use crate::io::SendError;
use crate::types::encoding::DecodeError;
use crate::types::flags::Flags;
use crate::types::message::MessageType;

#[derive(Clone, PartialEq, prost::Message, Error)]
#[error("Error code {}: {message}", code_to_str(*.code))]
pub struct Status {
    /// The status code, which should be an enum value of `Code`.
    #[prost(enumeration = "Code")]
    pub code: i32,

    /// A developer-facing error message, which should be in English. Any
    /// user-facing error message should be localized and sent in the
    /// `details` field, or localized by the client.
    #[prost(string)]
    pub message: String,

    /// A list of messages that carry the error details. There is a common set of
    /// message types for APIs to use.
    #[prost(message, repeated)]
    pub details: Vec<Any>,
}

impl Status {
    pub fn new(code: Code, message: impl Into<String>) -> Self {
        let code = code as i32;
        let message = message.into();
        let details = vec![];
        Self {
            code,
            message,
            details,
        }
    }

    fn invalid_argument(message: impl Into<String>) -> Self {
        Status {
            code: Code::InvalidArgument as i32,
            message: message.into(),
            details: vec![],
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Status {
            code: Code::NotFound as i32,
            message: message.into(),
            details: vec![],
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Status {
            code: Code::Internal as i32,
            message: message.into(),
            details: vec![],
        }
    }

    fn deadline_exceeded(message: impl Into<String>) -> Self {
        Status {
            code: Code::DeadlineExceeded as i32,
            message: message.into(),
            details: vec![],
        }
    }

    fn unknown(message: impl Into<String>) -> Self {
        Status {
            code: Code::Unknown as i32,
            message: message.into(),
            details: vec![],
        }
    }

    fn aborted(message: impl Into<String>) -> Self {
        Status {
            code: Code::Aborted as i32,
            message: message.into(),
            details: vec![],
        }
    }

    pub(crate) fn stream_in_use(stream_id: u32) -> Self {
        Self::invalid_argument(format!("Stream `{stream_id}` is already in use"))
    }

    pub(crate) fn invalid_stream_id(stream_id: u32) -> Self {
        Self::invalid_argument(format!("Stream id must be odd, found `{stream_id}`"))
    }

    pub(crate) fn stream_closed(stream_id: u32) -> Self {
        Self::invalid_argument(format!("Channel on stream `{stream_id}` is closed"))
    }

    pub(crate) fn channel_closed() -> Self {
        Self::aborted("Channel closed")
    }

    pub(crate) fn expected_request(stream_id: u32, ty: MessageType) -> Self {
        const TY: MessageType = MessageType::Request;
        let msg = format!("Invalid message type {ty:?} on stream `{stream_id}`, expected {TY:?}",);
        Self::invalid_argument(msg)
    }

    pub(crate) fn method_not_found(service: impl Display, method: impl Display) -> Self {
        let msg = format!("/{service}/{method} is not supported");
        Self::not_found(msg)
    }

    pub(crate) fn failed_to_encode(err: EncodeError) -> Self {
        Status::invalid_argument(format!("Error encoding message: {err}"))
    }

    pub(crate) fn failed_to_decode(err: DecodeError) -> Self {
        Status::invalid_argument(format!("Error decoding message: {err}"))
    }

    pub(crate) fn invalid_request_flags(expected: Flags, actual: Flags) -> Self {
        Status::invalid_argument(format!(
            "Invalid request flags. Expected {expected:?}, found {actual:?}"
        ))
    }

    pub(crate) fn timeout() -> Self {
        Status::deadline_exceeded("Request timed out")
    }

    pub fn from_error(err: impl std::error::Error) -> Self {
        Status::unknown(err.to_string())
    }
}

impl From<DecodeError> for Status {
    fn from(value: DecodeError) -> Self {
        Self::failed_to_decode(value)
    }
}

impl From<EncodeError> for Status {
    fn from(value: EncodeError) -> Self {
        Self::failed_to_encode(value)
    }
}

impl From<SendError> for Status {
    fn from(value: SendError) -> Self {
        Self::internal(format!("{value}"))
    }
}

fn code_to_str(code: i32) -> &'static str {
    let Ok(code) = Code::try_from(code) else {
        return "<None>";
    };
    code.as_str_name()
}
