use std::io::Result as IoResult;

use async_trait::async_trait;
use tokio::net::{UnixListener as TokioUnixListener, UnixStream};

use super::{Connection, Listener};

pub struct UnixListener {
    inner: TokioUnixListener,
    path: String,
}

#[async_trait]
impl Listener for UnixListener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        let (conn, _) = self.inner.accept().await?;
        Ok(Box::new(conn))
    }
}

pub fn bind(path: impl AsRef<str>) -> IoResult<UnixListener> {
    let path = path.as_ref().into();
    let inner = TokioUnixListener::bind(&path)?;
    Ok(UnixListener { inner, path })
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<UnixStream> {
    UnixStream::connect(addr.as_ref()).await
}

impl Drop for UnixListener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
