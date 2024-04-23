use std::io::{Error as IoError, ErrorKind, Result as IoResult};

use async_trait::async_trait;
use tokio_vsock::{VsockAddr, VsockListener, VsockStream};

pub struct Listener {
    inner: VsockListener,
}

#[async_trait]
impl super::Listener for Listener {
    async fn accept(&mut self) -> IoResult<Box<dyn super::Connection>> {
        let (conn, _) = self.inner.accept().await?;
        Ok(Box::new(conn))
    }
}

pub fn bind(addr: impl AsRef<str>) -> IoResult<Listener> {
    let addr = parse_vsock_addr(addr)?;
    let inner = VsockListener::bind(addr)?;
    Ok(Listener { inner })
}

pub async fn connect(addr: impl AsRef<str>) -> IoResult<VsockStream> {
    let addr = parse_vsock_addr(addr)?;
    VsockStream::connect(addr).await
}

fn parse_vsock_addr(addr: impl AsRef<str>) -> IoResult<VsockAddr> {
    let addr = addr.as_ref();
    let Some((cid, port)) = addr.split_once(':') else {
        return Err(IoError::new(
            ErrorKind::InvalidInput,
            format!("Invalid vsock address `{addr}`, address format should be `<cid>:<port>`"),
        ));
    };
    let cid = parse_u32(cid)?;
    let port = parse_u32(port)?;
    Ok(VsockAddr::new(cid, port))
}

fn parse_u32(num: &str) -> IoResult<u32> {
    let num = num.to_lowercase();
    let num = if let Some(num) = num.strip_prefix("0x") {
        u32::from_str_radix(num, 16)
    } else if let Some(num) = num.strip_prefix("0o") {
        u32::from_str_radix(num, 8)
    } else if let Some(num) = num.strip_prefix("0b") {
        u32::from_str_radix(num, 2)
    } else {
        num.parse()
    };
    num.map_err(|err| IoError::new(ErrorKind::InvalidInput, err))
}

impl From<VsockListener> for Listener {
    fn from(inner: VsockListener) -> Self {
        Self { inner }
    }
}