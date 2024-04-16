use std::io::Result as IoResult;

use async_trait::async_trait;
use tokio::net::{TcpListener as TokioTcpListener, TcpStream};

use super::{Connection, Listener};

pub struct TcpListener {
    inner: TokioTcpListener,
}

#[async_trait]
impl Listener for TcpListener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        let (conn, _) = self.inner.accept().await?;
        Ok(Box::new(conn))
    }
}

pub async fn bind(addr: impl AsRef<str>) -> IoResult<TcpListener> {
    let inner = TokioTcpListener::bind(addr.as_ref()).await?;
    Ok(TcpListener { inner })
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<TcpStream> {
    TcpStream::connect(addr.as_ref()).await
}
