use std::future::poll_fn;
use std::io::Result as IoResult;
use std::os::unix::net::{
    SocketAddr, UnixListener as StdUnixListener, UnixStream as StdUnixStream,
};

use async_trait::async_trait;
use tokio::net::{UnixListener, UnixStream};

use super::{Connection, Listener};

#[async_trait]
impl Listener for UnixListener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        let (conn, _) = UnixListener::accept(self).await?;
        Ok(Box::new(conn))
    }
}

pub fn bind(addr: impl AsRef<str>) -> IoResult<impl Listener> {
    let addr: String = addr.as_ref().into();
    let inner = {
        let addr = make_socket_addr(&addr)?;
        let inner = StdUnixListener::bind_addr(&addr)?;
        inner.set_nonblocking(true)?;
        UnixListener::from_std(inner)?
    };

    let addr = Some(addr);
    Ok(RaiiListener { inner, addr })
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<impl Connection> {
    let addr = make_socket_addr(addr.as_ref())?;
    let conn = StdUnixStream::connect_addr(&addr)?;
    conn.set_nonblocking(true)?;
    let conn = UnixStream::from_std(conn)?;
    poll_fn(|cx| conn.poll_write_ready(cx)).await?;
    Ok(conn)
}

struct RaiiListener {
    inner: UnixListener,
    addr: Option<String>,
}

#[async_trait]
impl Listener for RaiiListener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        let (conn, _) = UnixListener::accept(&self.inner).await?;
        Ok(Box::new(conn))
    }
}

impl Drop for RaiiListener {
    fn drop(&mut self) {
        if let Some(addr) = &self.addr {
            cleanup_socket(addr);
        }
    }
}

fn make_socket_addr(addr: &str) -> IoResult<SocketAddr> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if let Some(name) = addr.strip_prefix('@') {
        use std::os::linux::net::SocketAddrExt;
        return SocketAddr::from_abstract_name(name);
    }
    SocketAddr::from_pathname(addr)
}

fn cleanup_socket(addr: &str) {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if addr.strip_prefix('@').is_some() {
        return;
    }
    let _ = std::fs::remove_file(addr);
}
