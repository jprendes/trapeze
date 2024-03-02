pub trait AsyncRead: tokio::io::AsyncRead + Unpin + Send + Sync + 'static {}
pub trait AsyncWrite: tokio::io::AsyncWrite + Unpin + Send + Sync + 'static {}

impl<T: tokio::io::AsyncRead + Unpin + Send + Sync + 'static> AsyncRead for T {}
impl<T: tokio::io::AsyncWrite + Unpin + Send + Sync + 'static> AsyncWrite for T {}
