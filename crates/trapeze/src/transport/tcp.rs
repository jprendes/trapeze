use std::io::Result as IoResult;

use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};

pub struct Listener {
    inner: TcpListener,
}

#[async_trait]
impl super::Listener for Listener {
    async fn accept(&mut self) -> IoResult<Box<dyn super::Connection>> {
        let (conn, _) = self.inner.accept().await?;
        Ok(Box::new(conn))
    }
}

pub async fn bind(addr: impl AsRef<str>) -> IoResult<Listener> {
    let inner = TcpListener::bind(addr.as_ref()).await?;
    Ok(Listener { inner })
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<TcpStream> {
    TcpStream::connect(addr.as_ref()).await
}
