use std::cmp::min;

pub use prost::DecodeError;
use std::future::Future;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Clone, Debug)]
pub struct Encoded(Vec<u8>);

pub const MAX_DATA_LENGTH: usize = 4194304;

#[derive(thiserror::Error, Debug)]
pub enum EncodeError {
    #[error("Encoded buffer is too long ({0} bytes > {MAX_DATA_LENGTH} bytes)")]
    TooLong(usize),
}

impl Encoded {
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }

    fn check_size(size: usize) -> Result<(), EncodeError> {
        if size > MAX_DATA_LENGTH {
            return Err(EncodeError::TooLong(size));
        }
        Ok(())
    }

    pub fn buffer(data: impl Into<Vec<u8>> + AsRef<[u8]>) -> Result<Self, EncodeError> {
        Self::check_size(data.as_ref().len())?;
        Ok(Self(data.into()))
    }

    pub fn encode(data: &impl prost::Message) -> Result<Self, EncodeError> {
        Self::check_size(prost::Message::encoded_len(data))?;
        Ok(Self(prost::Message::encode_to_vec(data)))
    }

    pub fn decode<T: prost::Message + Sized + Default>(&self) -> Result<T, DecodeError> {
        prost::Message::decode(self.0.as_slice())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ReadEncodedError {
    #[error("{0}")]
    Encoded(#[from] EncodeError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

async fn discard_bytes<R: AsyncRead + Unpin + ?Sized>(
    reader: &mut R,
    mut n_bytes: usize,
) -> std::io::Result<()> {
    let mut buf = [0u8; 4096];
    while n_bytes > 0 {
        let bytes_to_read = min(buf.len(), n_bytes);
        n_bytes -= reader.read(&mut buf[..bytes_to_read]).await?;
    }
    Ok(())
}

pub trait AsyncReadEncoded: AsyncRead + Unpin + Send {
    fn read_encoded(
        &mut self,
        len: usize,
    ) -> impl Future<Output = Result<Encoded, ReadEncodedError>> + Send {
        async move {
            if let Err(err) = Encoded::check_size(len) {
                discard_bytes(self, len).await?;
                return Err(err.into());
            }
            let mut buf = vec![0; len];
            self.read_exact(&mut buf).await?;
            Ok(Encoded::buffer(buf)?)
        }
    }
}

impl<T: AsyncRead + Unpin + Send> AsyncReadEncoded for T {}
