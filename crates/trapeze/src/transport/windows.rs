use std::ffi::OsString;
use std::io::Result as IoResult;
use std::mem::replace;
use std::time::Duration;

use async_trait::async_trait;
use tokio::net::windows::named_pipe::{
    ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
};
use tokio::time::sleep;
use windows_sys::Win32::Foundation::ERROR_PIPE_BUSY;

pub struct Listener {
    inner: NamedPipeServer,
    name: OsString,
}

#[async_trait]
impl super::Listener for Listener {
    async fn accept(&mut self) -> IoResult<Box<dyn super::Connection>> {
        self.inner.connect().await?;

        let server = replace(&mut self.inner, ServerOptions::new().create(&self.name)?);

        Ok(Box::new(server))
    }
}

pub async fn bind(name: impl AsRef<str>) -> IoResult<Listener> {
    let name = name.as_ref().into();
    let inner = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&name)?;

    Ok(Listener { name, inner })
}

pub async fn connect(name: impl AsRef<str>) -> IoResult<NamedPipeClient> {
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
