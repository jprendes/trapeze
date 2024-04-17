use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::ops::DerefMut;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

mod tcp;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(all(unix, feature = "vsock"))]
mod vsock;

pub trait Connection: AsyncRead + AsyncWrite + Send + Unpin + 'static {}

impl<T: AsyncRead + AsyncWrite + Send + Unpin + 'static> Connection for T {}

#[async_trait]
pub trait Listener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>>;
}

#[async_trait]
impl Listener for Box<dyn Listener + Send> {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        self.deref_mut().accept().await
    }
}

pub async fn bind(addr: impl AsRef<str>) -> IoResult<Box<dyn Listener + Send>> {
    let addr = addr.as_ref();

    if let Some(addr) = addr.strip_prefix("tcp://") {
        return Ok(Box::new(tcp::bind(addr).await?));
    }

    #[cfg(all(unix, feature = "vsock"))]
    if let Some(addr) = addr.strip_prefix("vsock://") {
        return Ok(Box::new(vsock::bind(addr).await?));
    }

    #[cfg(unix)]
    if let Some(addr) = addr.strip_prefix("unix://") {
        return Ok(Box::new(unix::bind(addr)?));
    }

    #[cfg(windows)]
    if addr.starts_with(r"\\.\pipe\") {
        return Ok(Box::new(windows::bind(addr).await?));
    }

    Err(IoError::new(
        ErrorKind::Unsupported,
        format!("Scheme {addr:?} is not supported"),
    ))
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<Box<dyn Connection>> {
    let addr = addr.as_ref();

    if let Some(addr) = addr.strip_prefix("tcp://") {
        return Ok(Box::new(tcp::connect(addr).await?));
    }

    #[cfg(all(unix, feature = "vsock"))]
    if let Some(addr) = addr.strip_prefix("vsock://") {
        return Ok(Box::new(vsock::connect(addr).await?));
    }

    #[cfg(unix)]
    if let Some(addr) = addr.strip_prefix("unix://") {
        return Ok(Box::new(unix::connect(addr).await?));
    }

    #[cfg(windows)]
    if addr.starts_with(r"\\.\pipe\") {
        return Ok(Box::new(windows::connect(addr).await?));
    }

    Err(IoError::new(
        ErrorKind::Unsupported,
        format!("Scheme {addr:?} is not supported"),
    ))
}
