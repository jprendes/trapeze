use std::borrow::Cow;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("Invalid input: {0}")]
pub struct InvalidInput(pub Cow<'static, str>);

#[derive(Error, Debug, Clone)]
pub enum EncodeError {
    #[error("Error encoding message: {0}")]
    InvalidInput(#[from] InvalidInput),

    #[error("Insufficient buffer capacity ({required} bytes > {capacity} bytes)")]
    InsuficientCapacity { required: usize, capacity: usize },
}

#[derive(Error, Debug, Clone)]
pub enum DecodeError {
    #[error("Unexpected EOF reading input byffer")]
    UnexpectedEof,

    #[error("Remaining bytes in input buffer: {0} bytes")]
    RemainingBytes(usize),

    #[error("Error decoding message: {0}")]
    InvalidInput(#[from] InvalidInput),

    #[error("Error decoding message: Invalid protobuf stream: {0}")]
    InvalidProtobufStream(#[from] prost::DecodeError),
}

impl<T: Into<Cow<'static, str>>> From<T> for InvalidInput {
    fn from(msg: T) -> Self {
        Self(msg.into())
    }
}

impl From<prost::EncodeError> for EncodeError {
    fn from(err: prost::EncodeError) -> Self {
        EncodeError::InsuficientCapacity {
            required: err.required_capacity(),
            capacity: err.remaining(),
        }
    }
}

impl From<InvalidInput> for std::io::Error {
    fn from(value: InvalidInput) -> Self {
        IoError::new(IoErrorKind::InvalidInput, value)
    }
}
