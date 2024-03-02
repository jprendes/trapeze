use once_cell::sync::Lazy;

use crate::encoded::Encoded;
use crate::grpc::Response;
use crate::{Code, Status};

pub(crate) static REQUEST_TIMEOUT_STATUS: Lazy<Status> =
    Lazy::new(|| Status::new(Code::DeadlineExceeded, "Request timeout"));

pub(crate) static REQUEST_TIMEOUT_ENCODED: Lazy<Encoded> = Lazy::new(|| {
    let response = Response {
        status: Some(REQUEST_TIMEOUT_STATUS.clone()),
        payload: vec![],
    };

    // Safe because the response serializes to less than 4MB
    Encoded::encode(&response).unwrap()
});

pub(crate) static RESPONSE_TOO_LONG_ENCODED: Lazy<Encoded> = Lazy::new(|| {
    let response = Response {
        status: Some(Status::new(Code::Internal, "Response too long")),
        payload: vec![],
    };

    // Safe because the response serializes to less than 4MB
    Encoded::encode(&response).unwrap()
});
