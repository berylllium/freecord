use std::{fmt::Debug, io};

use futures::{Stream, stream};
use iroh::endpoint::{Connection, ConnectionError, RecvStream, SendStream};
use serde::{Serialize, de::DeserializeOwned};
use util::{AsyncReadVarintExt, WriteVarintExt};

/// Default max message size (16 MiB)
pub const MAX_MESSAGE_SIZE: u64 = 1024 * 1024 * 16;

/// Error code for iroh streams if the max message size was exceeded.
pub const ERROR_CODE_MAX_MESSAGE_SIZE_EXCEEDED: u32 = 1;

/// Error code for iroh streams if postcard serialization failed.
pub const ERROR_CODE_INVALID_POSTCARD: u32 = 2;

pub trait Message: Debug + Serialize + DeserializeOwned + Send + Sync + Unpin + 'static {}

impl<T> Message for T where T: Debug + Serialize + DeserializeOwned + Send + Sync + Unpin + 'static {}

pub async fn open_bi<T: Message>(
    connection: &Connection,
) -> Result<(Sender<T>, Receiver<T>), ConnectionError> {
    let (send, recv) = connection.open_bi().await?;

    Ok((
        Sender {
            send,
            buffer: Vec::with_capacity(100),
            _marker: std::marker::PhantomData,
        },
        Receiver {
            recv,
            _marker: std::marker::PhantomData,
        },
    ))
}

pub async fn accept_bi<T: Message>(
    connection: &Connection,
) -> Result<(Sender<T>, Receiver<T>), ConnectionError> {
    let (send, recv) = connection.accept_bi().await?;

    Ok((
        Sender {
            send,
            buffer: Vec::with_capacity(100),
            _marker: std::marker::PhantomData,
        },
        Receiver {
            recv,
            _marker: std::marker::PhantomData,
        },
    ))
}

pub struct Sender<T> {
    send: SendStream,
    buffer: Vec<u8>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Message> Sender<T> {
    pub async fn send(&mut self, value: T) -> Result<(), SendError> {
        let size = match postcard::experimental::serialized_size(&value) {
            Ok(size) => size,
            Err(e) => {
                self.send.reset(ERROR_CODE_INVALID_POSTCARD.into()).ok();
                return Err(SendError::Io(io::Error::new(io::ErrorKind::InvalidData, e)));
            }
        };

        if size as u64 > MAX_MESSAGE_SIZE {
            self.send
                .reset(ERROR_CODE_MAX_MESSAGE_SIZE_EXCEEDED.into())
                .ok();
            return Err(SendError::MaxMessageSizeExceeded);
        }

        let value = value;
        self.buffer.clear();
        if let Err(e) = self.buffer.write_length_prefixed(value) {
            self.send.reset(ERROR_CODE_INVALID_POSTCARD.into()).ok();
            return Err(e.into());
        }
        self.send.write_all(&self.buffer).await?;
        self.buffer.clear();

        Ok(())
    }
}

impl<T> Debug for Sender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sender").finish()
    }
}

pub struct Receiver<T> {
    recv: RecvStream,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Message> Receiver<T> {
    pub async fn recv(&mut self) -> Result<Option<T>, RecvError> {
        let read = &mut self.recv;

        let Some(size) = read.read_varint_u64().await? else {
            return Ok(None);
        };

        if size as u64 > MAX_MESSAGE_SIZE {
            self.recv
                .stop(ERROR_CODE_MAX_MESSAGE_SIZE_EXCEEDED.into())
                .ok();
            return Err(RecvError::MaxMessageSizeExceeded);
        }

        let mut buf = vec![0; size as usize];
        read.read_exact(&mut buf)
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::UnexpectedEof, e))?;
        let msg: T = postcard::from_bytes(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(Some(msg))
    }

    pub fn into_stream(self) -> impl Stream<Item = Result<T, RecvError>> + Send + Sync + 'static {
        stream::unfold(self, |mut recv| async move {
            recv.recv().await.transpose().map(|msg| (msg, recv))
        })
    }
}

impl<T> Debug for Receiver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Receiver").finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Maximum message size exceeded")]
    MaxMessageSizeExceeded,
    #[error(transparent)]
    WriteError(#[from] iroh::endpoint::WriteError),
}

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Maximum message size exceeded")]
    MaxMessageSizeExceeded,
}

mod util {
    use serde::{Serialize, de::DeserializeOwned};
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    /// Reads and parses a varint u64 from the reader.
    ///
    /// Returns Ok(None) if the sender has been dropped or the connection was closed.
    pub async fn read_varint_u64<R>(reader: &mut R) -> std::io::Result<Option<u64>>
    where
        R: AsyncRead + Unpin,
    {
        let mut result = 0u64;
        let mut shift = 0u32;

        loop {
            // We can only shift up to 63 bits
            if shift >= 64 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Varint is too large for u64",
                ));
            }

            // Read a single byte
            let res = reader.read_u8().await;
            if shift == 0 {
                if let Err(cause) = res {
                    if cause.kind() == std::io::ErrorKind::UnexpectedEof {
                        return Ok(None);
                    } else {
                        return Err(cause);
                    }
                }
            }

            let byte = res?;

            // Extract he 7 value bits
            let value = (byte & 0b0111_1111) as u64;

            // Add the bits to our result
            result |= value << shift;

            // If the high bit is not set, this is the last byte
            if byte & 0b1000_0000 == 0 {
                break;
            }

            // Move to the next 7 bits.
            shift += 7;
        }

        Ok(Some(result))
    }

    pub fn write_varint_u64_sync(
        value: u64,
        writer: &mut impl std::io::Write,
    ) -> std::io::Result<usize> {
        // Handle zero as a special case
        if value == 0 {
            writer.write_all(&[0])?;
            return Ok(1);
        }

        let mut bytes_written = 0;
        let mut remaining = value;

        while remaining > 0 {
            // Extract the 7 least significant bits
            let mut byte = (remaining & 0b0111_1111) as u8;
            remaining >>= 7;

            // Set the continuation but if there's more data
            if remaining > 0 {
                byte |= 0b1000_0000;
            }

            writer.write_all(&[byte])?;
            bytes_written += 1;
        }

        Ok(bytes_written)
    }

    pub fn write_length_prefixed<T: Serialize>(
        value: T,
        mut write: impl std::io::Write,
    ) -> std::io::Result<()> {
        let size = postcard::experimental::serialized_size(&value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?
            as u64;

        write_varint_u64_sync(size, &mut write)?;

        postcard::to_io(&value, &mut write)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(())
    }

    pub trait AsyncReadVarintExt: AsyncRead + Unpin {
        async fn read_varint_u64(&mut self) -> std::io::Result<Option<u64>>;

        async fn read_length_prefixed<T: DeserializeOwned>(
            &mut self,
            max_size: u64,
        ) -> std::io::Result<T>;
    }

    impl<T: AsyncRead + Unpin> AsyncReadVarintExt for T {
        async fn read_varint_u64(&mut self) -> std::io::Result<Option<u64>> {
            read_varint_u64(self).await
        }

        async fn read_length_prefixed<I: DeserializeOwned>(
            &mut self,
            max_size: u64,
        ) -> std::io::Result<I> {
            let size = match self.read_varint_u64().await? {
                Some(size) => size,
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "EOF reached",
                    ));
                }
            };

            if size > max_size {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Length-prefixed value too large",
                ));
            }

            let mut buf = vec![0; size as usize];
            self.read_exact(&mut buf).await?;
            postcard::from_bytes(&buf)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        }
    }

    pub trait WriteVarintExt: std::io::Write {
        fn write_varint_u64(&mut self, value: u64) -> std::io::Result<usize>;

        fn write_length_prefixed<T: Serialize>(&mut self, value: T) -> std::io::Result<()>;
    }

    impl<T: std::io::Write> WriteVarintExt for T {
        fn write_varint_u64(&mut self, value: u64) -> std::io::Result<usize> {
            write_varint_u64_sync(value, self)
        }

        fn write_length_prefixed<V: Serialize>(&mut self, value: V) -> std::io::Result<()> {
            write_length_prefixed(value, self)
        }
    }

    pub trait AsyncWriteVarintExt: AsyncWrite + Unpin {
        fn write_varint_u64(&mut self, value: u64) -> impl Future<Output = std::io::Result<usize>>;

        fn write_length_prefixed<T: Serialize>(
            &mut self,
            value: T,
        ) -> impl Future<Output = std::io::Result<usize>>;
    }

    impl<T: AsyncWrite + Unpin> AsyncWriteVarintExt for T {
        async fn write_varint_u64(&mut self, value: u64) -> std::io::Result<usize> {
            let mut buf = Vec::with_capacity(10);
            write_varint_u64_sync(value, &mut buf).unwrap();
            self.write_all(&buf[..]).await?;
            Ok(buf.len())
        }

        async fn write_length_prefixed<V: Serialize>(
            &mut self,
            value: V,
        ) -> std::io::Result<usize> {
            let mut buf = Vec::new();
            write_length_prefixed(value, &mut buf)?;
            let size = buf.len();
            self.write_all(&buf).await?;
            Ok(size)
        }
    }
}
