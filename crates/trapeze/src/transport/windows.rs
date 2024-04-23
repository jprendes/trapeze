use std::ffi::OsString;
use std::io::Result as IoResult;
use std::mem::replace;
use std::time::Duration;

use async_trait::async_trait;
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeServer, ServerOptions,
};
use tokio::time::sleep;
use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

use super::{Connection, Listener};

pub struct NamedPipeListener {
    inner: NamedPipeServer,
    name: OsString,
}

impl NamedPipeListener {
    pub async fn bind(name: impl AsRef<str>) -> IoResult<Self> {
        let name = name.as_ref().into();
        let inner = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&name)?;

        Ok(Self { name, inner })
    }
}

#[async_trait]
impl Listener for NamedPipeListener {
    async fn accept(&mut self) -> IoResult<Box<dyn Connection>> {
        self.inner.connect().await?;

        let server = replace(&mut self.inner, ServerOptions::new().create(&self.name)?);

        Ok(Box::new(server))
    }
}

pub async fn bind(name: impl AsRef<str>) -> IoResult<impl Listener> {
    NamedPipeListener::bind(name).await
}

pub async fn connect(name: impl AsRef<str>) -> IoResult<impl Connection> {
    let name = name.as_ref();
    let client = loop {
        match ClientOptions::new().open(name) {
            Ok(client) => break client,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) => (),
            Err(e) => return Err(e),
        }
        sleep(Duration::from_millis(50)).await;
    };
    Ok(client)
}
