use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// Bounds a bad length prefix from a malicious/buggy peer — without this,
// a crafted 4-byte header claiming a 4 GB message triggers a 4 GB
// allocation before you've even authenticated who's talking to you.
const MAX_MESSAGE_SIZE: u32 = 64 * 1024 * 1024; // 64 MB

#[derive(thiserror::Error, Debug)]
pub enum FramingError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("message of {0} bytes exceeds the {MAX_MESSAGE_SIZE} byte limit")]
    TooLarge(u32),
    #[error("serialization error: {0}")]
    Serde(#[from] bincode::Error),
}

pub async fn send_msg<W, T>(writer: &mut W, msg: &T) -> Result<(), FramingError>
where
    W: AsyncWrite + Unpin,
    T: serde::Serialize,
{
    let bytes = bincode::serialize(msg)?;
    writer.write_u32(bytes.len() as u32).await?;
    writer.write_all(&bytes).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn recv_msg<R, T>(reader: &mut R) -> Result<T, FramingError>
where
    R: AsyncRead + Unpin,
    T: serde::de::DeserializeOwned,
{
    let len = reader.read_u32().await?;
    if len > MAX_MESSAGE_SIZE {
        return Err(FramingError::TooLarge(len));
    }
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf).await?;
    Ok(bincode::deserialize(&buf)?)
}