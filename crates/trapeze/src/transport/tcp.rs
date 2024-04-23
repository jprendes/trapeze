use std::io::Result as IoResult;

use async_trait::async_trait;
use tokio::net::{TcpListener, TcpStream};

use super::{Connection, Listener};

#[async_trait]
impl Listener for TcpListener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        let (conn, _) = TcpListener::accept(self).await?;
        Ok(Box::new(conn))
    }
}

pub async fn bind(addr: impl AsRef<str>) -> IoResult<impl Listener> {
    TcpListener::bind(addr.as_ref()).await
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<impl Connection> {
    TcpStream::connect(addr.as_ref()).await
}
