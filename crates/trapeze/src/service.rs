use std::future::Future;
use std::pin::Pin;

use crate::encoded::Encoded;
use crate::{Code, Result, Status};

pub trait Service: Send + Sync {
    fn name(&self) -> &'static str;
    fn dispatch<'a, 'b>(
        &'a self,
        method: &'b str,
        _payload: Encoded,
    ) -> Pin<Box<dyn Future<Output = Result<Encoded>> + Send + 'a>>
    where
        Self: Sync + 'a,
        'b: 'a,
    {
        Box::pin(async move {
            Ok(Encoded::encode(&Status::new(
                Code::NotFound,
                format!("/{}/{method} is not supported", self.name()),
            ))?)
        })
    }
}
