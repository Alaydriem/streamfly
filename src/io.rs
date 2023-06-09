use anyhow::Result;
use futures::{AsyncReadExt, AsyncWriteExt};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Reader, Writer};

pub(crate) async fn read_packet<T>(r: &mut Reader) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf).await?;
    let length = u32::from_le_bytes(buf) as usize;
    let mut buf = vec![0u8; length];
    r.read_exact(&mut buf).await?;
    let res = serde_json::from_slice::<T>(&buf)?;
    Ok(res)
}

pub(crate) async fn write_packet<T: Serialize>(w: &mut Writer, packet: T) -> Result<()> {
    let buf = serde_json::to_vec(&packet)?;
    let length = buf.len() as u32;
    w.write_all(&u32::to_le_bytes(length)).await?;
    w.write_all(&buf).await?;
    Ok(())
}
