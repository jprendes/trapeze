use std::future::Future;
use std::ops::Deref;
use std::time::Duration;

use crate::{Result, Status};

#[derive(Clone, Copy, Debug)]
pub enum Timeout {
    None,
    Duration(Duration),
}

impl Default for Timeout {
    fn default() -> Self {
        Self::None
    }
}

impl Deref for Timeout {
    type Target = Duration;
    fn deref(&self) -> &Self::Target {
        match self {
            Timeout::None => &Duration::ZERO,
            Timeout::Duration(t) => t,
        }
    }
}

pub trait WithTimeout {
    type Ouput;
    fn with_timeout(self, t: impl Into<Timeout>) -> impl Future<Output = Result<Self::Ouput>>;
}

impl<Output, F: Future<Output = Result<Output>>> WithTimeout for F {
    type Ouput = Output;
    async fn with_timeout(self, t: impl Into<Timeout>) -> Result<Self::Ouput> {
        let Timeout::Duration(t) = t.into() else {
            return self.await;
        };

        let Ok(output) = tokio::time::timeout(t, self).await else {
            return Err(Status::timeout());
        };

        output
    }
}

const MAX_TIMEOUT: Duration = Duration::from_nanos(i64::MAX as u64);

impl From<Option<Duration>> for Timeout {
    fn from(value: Option<Duration>) -> Self {
        match value {
            Some(t) => t.into(),
            _ => Timeout::None,
        }
    }
}

impl From<Duration> for Timeout {
    fn from(t: Duration) -> Self {
        if t.is_zero() {
            return Timeout::None;
        }
        Timeout::Duration(t.min(MAX_TIMEOUT))
    }
}

impl Timeout {
    pub fn from_nanos(nanos: i64) -> Self {
        Some(Duration::from_nanos(nanos.max(0) as u64)).into()
    }

    pub fn as_nanos(&self) -> i64 {
        self.deref().as_nanos().min(i64::MAX as u128) as i64
    }
}
