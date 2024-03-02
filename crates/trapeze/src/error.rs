pub use crate::grpc::{Code, Status};

impl Status {
    pub fn new(code: Code, message: impl ToString) -> Self {
        Self {
            code: code.into(),
            message: message.to_string(),
            details: vec![],
        }
    }
}

impl<T: std::error::Error> From<T> for Status {
    fn from(value: T) -> Self {
        Self::new(Code::Unknown, &value)
    }
}
