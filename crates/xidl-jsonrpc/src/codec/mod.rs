#[cfg(test)]
mod test;

use crate::Error;
use serde::Serialize;
use serde::de::DeserializeOwned;
#[cfg(feature = "msgpack")]
use tokio::io::AsyncReadExt;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

/// Wire codec used to frame JSON-RPC messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Codec {
    /// Newline-delimited JSON, as defined by JSON-RPC 2.0.
    Json,
    /// Length-prefixed MessagePack, available with the `msgpack` feature.
    #[cfg(feature = "msgpack")]
    Msgpack,
}

impl Codec {
    /// Writes `value` to `writer` using this codec's framing.
    pub(crate) async fn write<W, T>(self, writer: &mut W, value: &T) -> Result<(), Error>
    where
        W: AsyncWrite + Unpin,
        T: Serialize,
    {
        match self {
            Self::Json => write_json_line(writer, value).await,
            #[cfg(feature = "msgpack")]
            Self::Msgpack => write_msgpack_frame(writer, value).await,
        }
    }

    /// Reads the next value from `reader`, returning `None` on a clean EOF.
    pub(crate) async fn read<R, T>(self, reader: &mut R) -> Result<Option<T>, Error>
    where
        R: AsyncBufRead + Unpin,
        T: DeserializeOwned,
    {
        match self {
            Self::Json => read_json_line(reader).await,
            #[cfg(feature = "msgpack")]
            Self::Msgpack => read_msgpack_frame(reader).await,
        }
    }
}

async fn write_json_line<W, T>(writer: &mut W, value: &T) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let payload = serde_json::to_string(value)?;
    writer.write_all(payload.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

async fn read_json_line<R, T>(reader: &mut R) -> Result<Option<T>, Error>
where
    R: AsyncBufRead + Unpin,
    T: DeserializeOwned,
{
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&line)?))
}

#[cfg(feature = "msgpack")]
async fn write_msgpack_frame<W, T>(writer: &mut W, value: &T) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let mut payload = Vec::new();
    let mut serializer = rmp_serde::Serializer::new(&mut payload).with_struct_map();
    value
        .serialize(&mut serializer)
        .map_err(|err| Error::Msgpack(err.to_string()))?;
    let len = u32::try_from(payload.len())
        .map_err(|_| Error::Protocol("msgpack frame exceeds u32 length"))?;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&payload).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(feature = "msgpack")]
async fn read_msgpack_frame<R, T>(reader: &mut R) -> Result<Option<T>, Error>
where
    R: AsyncBufRead + Unpin,
    T: DeserializeOwned,
{
    let mut len = [0_u8; 4];
    let mut first = [0_u8; 1];
    let n = reader.read(&mut first).await?;
    if n == 0 {
        return Ok(None);
    }
    len[0] = first[0];
    reader.read_exact(&mut len[1..]).await?;
    let payload_len = usize::try_from(u32::from_be_bytes(len))
        .map_err(|_| Error::Protocol("msgpack frame length overflow"))?;
    let mut payload = vec![0_u8; payload_len];
    reader.read_exact(&mut payload).await?;
    let value = rmp_serde::from_slice(&payload).map_err(|err| Error::Msgpack(err.to_string()))?;
    Ok(Some(value))
}
