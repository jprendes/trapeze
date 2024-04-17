use std::io::Result as IoResult;
use std::os::unix::net::{SocketAddr, UnixListener as StdUnixListener, UnixStream as StdUnixStream};

use async_trait::async_trait;
use tokio::net::{UnixListener as TokioUnixListener, UnixStream as TokioUnixStream};

use super::{Connection, Listener};

pub struct UnixListener {
    inner: TokioUnixListener,
    addr: String,
}

#[async_trait]
impl Listener for UnixListener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        let (conn, _) = self.inner.accept().await?;
        Ok(Box::new(conn))
    }
}

pub fn bind(addr: impl AsRef<str>) -> IoResult<UnixListener> {
    let addr: String = addr.as_ref().into();
    let inner = {
        let addr = make_socket_addr(&addr)?;
        let inner = StdUnixListener::bind_addr(&addr)?;
        inner.set_nonblocking(true)?;
        TokioUnixListener::from_std(inner)?
    };

    Ok(UnixListener { inner, addr })
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<TokioUnixStream> {
    let addr = make_socket_addr(addr.as_ref())?;
    let inner = StdUnixStream::connect_addr(&addr)?;
    inner.set_nonblocking(true)?;
    TokioUnixStream::from_std(inner)
}

impl Drop for UnixListener {
    fn drop(&mut self) {
        cleanup_socket(&self.addr);
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
