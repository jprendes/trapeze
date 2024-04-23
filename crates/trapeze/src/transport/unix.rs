use std::future::poll_fn;
use std::io::Result as IoResult;
use std::os::unix::net::{
    SocketAddr, UnixListener as StdUnixListener, UnixStream as StdUnixStream,
};

use async_trait::async_trait;
use tokio::net::{UnixListener, UnixStream};

pub struct Listener {
    inner: UnixListener,
    addr: Option<String>,
}

#[async_trait]
impl super::Listener for Listener {
    async fn accept(&mut self) -> IoResult<Box<dyn super::Connection>> {
        let (conn, _) = self.inner.accept().await?;
        Ok(Box::new(conn))
    }
}

pub fn bind(addr: impl AsRef<str>) -> IoResult<Listener> {
    let addr: String = addr.as_ref().into();
    let inner = {
        let addr = make_socket_addr(&addr)?;
        let inner = StdUnixListener::bind_addr(&addr)?;
        inner.set_nonblocking(true)?;
        UnixListener::from_std(inner)?
    };

    let addr = Some(addr);
    Ok(Listener { inner, addr })
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<UnixStream> {
    let addr = make_socket_addr(addr.as_ref())?;
    let inner = StdUnixStream::connect_addr(&addr)?;
    inner.set_nonblocking(true)?;
    let inner = UnixStream::from_std(inner)?;
    poll_fn(|cx| inner.poll_write_ready(cx)).await?;
    Ok(inner)
}

impl Drop for Listener {
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

impl From<UnixListener> for Listener {
    fn from(inner: UnixListener) -> Self {
        let addr = None;
        Self { inner, addr }
    }
}