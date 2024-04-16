use std::ops::Deref;
use std::time::Duration;

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
    #[must_use]
    pub fn from_nanos(nanos: i64) -> Self {
        let nanos = u64::try_from(nanos).unwrap_or(0);
        Some(Duration::from_nanos(nanos)).into()
    }

    #[must_use]
    pub fn as_nanos(&self) -> i64 {
        let nanos = self.deref().as_nanos();
        i64::try_from(nanos).unwrap_or(i64::MAX)
    }
}
