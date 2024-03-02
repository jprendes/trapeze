use std::collections::HashMap;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;
use std::usize;

use crate::grpc::KeyValue;

#[derive(Default, Clone, Debug)]
pub struct Metadata(HashMap<String, Vec<String>>);

impl Deref for Metadata {
    type Target = HashMap<String, Vec<String>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Metadata {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K: ToString, V: ToString> From<&(K, V)> for KeyValue {
    fn from(value: &(K, V)) -> Self {
        KeyValue {
            key: value.0.to_string(),
            value: value.1.to_string(),
        }
    }
}

impl From<&KeyValue> for KeyValue {
    fn from(value: &KeyValue) -> Self {
        value.clone()
    }
}

impl<T, const N: usize> From<[T; N]> for Metadata
where
    for<'a> &'a T: Into<KeyValue>,
{
    fn from(value: [T; N]) -> Self {
        value.as_slice().into()
    }
}

impl<T, const N: usize> From<&[T; N]> for Metadata
where
    for<'a> &'a T: Into<KeyValue>,
{
    fn from(value: &[T; N]) -> Self {
        value.as_slice().into()
    }
}

impl<T> From<&[T]> for Metadata
where
    for<'a> &'a T: Into<KeyValue>,
{
    fn from(metadata: &[T]) -> Self {
        let mut map: HashMap<String, Vec<String>> = HashMap::default();
        for kv in metadata.iter() {
            let KeyValue { key, value } = kv.into();
            match map.get_mut(&key) {
                Some(v) => {
                    v.push(value.clone());
                }
                None => {
                    map.insert(key.clone(), vec![value.clone()]);
                }
            }
        }
        Self(map)
    }
}

impl From<Metadata> for Vec<KeyValue> {
    fn from(metadata: Metadata) -> Self {
        metadata
            .iter()
            .flat_map(|(k, v)| {
                v.iter().map(|v| KeyValue {
                    key: k.clone(),
                    value: v.clone(),
                })
            })
            .collect()
    }
}

impl From<HashMap<String, Vec<String>>> for Metadata {
    fn from(metadata: HashMap<String, Vec<String>>) -> Self {
        Self(metadata)
    }
}

impl From<Option<HashMap<String, Vec<String>>>> for Metadata {
    fn from(metadata: Option<HashMap<String, Vec<String>>>) -> Self {
        Self(metadata.unwrap_or_default())
    }
}

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
    pub fn from_nanos(nanos: i64) -> Self {
        Some(Duration::from_nanos(nanos.max(0) as u64)).into()
    }

    pub fn as_nanos(&self) -> i64 {
        self.deref().as_nanos().min(i64::MAX as u128) as i64
    }
}

#[derive(Default, Clone, Debug)]
pub struct Context {
    pub metadata: Metadata,
    pub timeout: Timeout,
}

tokio::task_local! {
    pub(crate) static CONTEXT: Arc<Context>;
}

pub fn get_context() -> Arc<Context> {
    CONTEXT.with(|c| c.clone())
}

pub fn try_get_context() -> Option<Arc<Context>> {
    CONTEXT.try_with(|c| c.clone()).ok()
}

pub fn with_context<T>(ctx: Context, f: impl Future<Output = T>) -> impl Future<Output = T> {
    CONTEXT.scope(Arc::new(ctx), f)
}
