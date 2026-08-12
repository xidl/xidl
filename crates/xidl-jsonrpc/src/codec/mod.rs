#[cfg(test)]
mod test;

use crate::Error;
use serde::Serialize;
use serde::de::DeserializeOwned;
#[cfg(feature = "msgpack")]
use tokio::io::AsyncReadExt;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

/// Maximum accepted wire-frame payload, in bytes, for both JSON and MessagePack.
const MAX_FRAME_LEN: usize = 4 * 1024 * 1024;

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
    let mut buf = Vec::new();
    loop {
        let step = {
            let available = reader.fill_buf().await?;
            if available.is_empty() {
                if buf.is_empty() {
                    return Ok(None);
                }
                return Ok(Some(serde_json::from_slice(&buf)?));
            }
            match available.iter().position(|byte| *byte == b'\n') {
                Some(index) => {
                    buf.extend_from_slice(&available[..index]);
                    ReadStep::Line(index + 1)
                }
                None => {
                    if buf.len() + available.len() > MAX_FRAME_LEN {
                        ReadStep::Oversized
                    } else {
                        buf.extend_from_slice(available);
                        ReadStep::Chunk(available.len())
                    }
                }
            }
        };
        match step {
            ReadStep::Line(consumed) => {
                reader.consume(consumed);
                if buf.len() > MAX_FRAME_LEN {
                    return Err(Error::FrameTooLarge {
                        max: MAX_FRAME_LEN,
                        framing: "json",
                    });
                }
                return Ok(Some(serde_json::from_slice(&buf)?));
            }
            ReadStep::Chunk(consumed) => reader.consume(consumed),
            ReadStep::Oversized => {
                drain_to_newline(reader).await?;
                return Err(Error::FrameTooLarge {
                    max: MAX_FRAME_LEN,
                    framing: "json",
                });
            }
        }
    }
}

/// Outcome of inspecting one buffered chunk while reading a JSON line.
enum ReadStep {
    /// The chunk ended at a newline; the value is the number of bytes to consume.
    Line(usize),
    /// The chunk contained no newline; the value is the number of bytes to consume.
    Chunk(usize),
    /// The frame already exceeds the maximum length; drain to the next newline.
    Oversized,
}

/// Consumes buffered input up to and including the next newline, if one is
/// already buffered.
///
/// This resynchronizes the reader after an oversized JSON line so the
/// following call can parse the next well-formed line instead of a partial
/// fragment. It never waits for a newline that has not arrived yet: when the
/// buffer holds only a fragment, that fragment is consumed and the caller
/// retries on the next read, keeping the server responsive to an unterminated
/// oversized line instead of blocking until EOF.
async fn drain_to_newline<R>(reader: &mut R) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
{
    let consumed = {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return Ok(());
        }
        available
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .unwrap_or(available.len())
    };
    reader.consume(consumed);
    Ok(())
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
    if payload_len > MAX_FRAME_LEN {
        drain_msgpack_payload(reader, payload_len).await?;
        return Err(Error::FrameTooLarge {
            max: MAX_FRAME_LEN,
            framing: "msgpack",
        });
    }
    let mut payload = vec![0_u8; payload_len];
    reader.read_exact(&mut payload).await?;
    let value = rmp_serde::from_slice(&payload).map_err(|err| Error::Msgpack(err.to_string()))?;
    Ok(Some(value))
}

/// Consumes exactly `len` bytes from the reader, stopping early on EOF.
///
/// A length-prefixed frame whose declared payload exceeds the limit still
/// occupies exactly `len` bytes on the wire, so those bytes are drained
/// before reporting `FrameTooLarge` to keep the reader aligned with the next
/// frame. A peer can declare a huge length and force a long drain; that is
/// accepted as parity with the JSON framing path.
#[cfg(feature = "msgpack")]
async fn drain_msgpack_payload<R>(reader: &mut R, len: usize) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
{
    let mut remaining = len;
    while remaining > 0 {
        let consumed = {
            let available = reader.fill_buf().await?;
            if available.is_empty() {
                break;
            }
            available.len().min(remaining)
        };
        reader.consume(consumed);
        remaining -= consumed;
    }
    Ok(())
}
