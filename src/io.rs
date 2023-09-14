use std::task::Poll;

use anyhow::{bail, Result};
use futures::{future::poll_fn, AsyncReadExt, AsyncWriteExt};
use nanoid::nanoid;
use s2n_quic::{
    connection::Handle,
    provider::datagram::default::{Receiver, Sender},
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    msg::{MsgAck, MsgRequest, MsgType},
    Reader, Writer,
};

pub(crate) async fn read_packet<T>(r: &mut Reader) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut buf = [0u8; 4];
    r.read_exact(&mut buf).await?;
    let length = u32::from_le_bytes(buf) as usize;
    let mut buf = vec![0u8; length];
    r.read_exact(&mut buf).await?;
    let res = rmp_serde::from_slice::<T>(&buf)?;
    Ok(res)
}

pub(crate) async fn write_packet<T: Serialize>(w: &mut Writer, packet: &T) -> Result<()> {
    let buf = rmp_serde::to_vec_named(&packet)?;
    let length = buf.len() as u32;
    w.write_all(&u32::to_le_bytes(length)).await?;
    w.write_all(&buf).await?;
    Ok(())
}

pub(crate) async fn send_request(
    handle: &Handle,
    msg_type: MsgType,
    payload: Vec<u8>,
) -> Result<()> {
    let tid = nanoid!();
    let msg = MsgRequest {
        tid: tid.to_owned(),
        msg_type,
        payload,
    };
    let buf = rmp_serde::to_vec_named(&msg)?;
    handle.datagram_mut(|sender: &mut Sender| {
        if let Err(e) = sender.send_datagram(buf.into()) {
            bail!(e);
        }
        anyhow::Ok(())
    })??;

    match poll_fn(|cx| {
        match handle.datagram_mut(|recv: &mut Receiver| recv.poll_recv_datagram(cx)) {
            Ok(value) => value.map(Ok),
            Err(err) => Poll::Ready(Err(err)),
        }
    })
    .await?
    {
        Ok(buf) => {
            let msg: MsgAck = rmp_serde::from_slice(&buf)?;
            if msg.tid != tid {
                bail!("tid mismatched");
            }
            if !msg.error.is_empty() {
                bail!(msg.error);
            }
        }
        Err(e) => {
            bail!(e);
        }
    }

    Ok(())
}

pub(crate) async fn recv_request(handle: &Handle) -> Result<MsgRequest> {
    match poll_fn(|cx| {
        match handle.datagram_mut(|recv: &mut Receiver| recv.poll_recv_datagram(cx)) {
            Ok(value) => value.map(Ok),
            Err(err) => Poll::Ready(Err(err)),
        }
    })
    .await?
    {
        Ok(buf) => {
            let req: MsgRequest = rmp_serde::from_slice(&buf)?;
            send_ack(&handle, &req.tid).await?;
            Ok(req)
        }
        Err(e) => {
            bail!(e);
        }
    }
}

async fn send_ack(handle: &Handle, tid: &str) -> Result<()> {
    let ack = MsgAck {
        tid: tid.to_owned(),
        error: String::from(""),
    };
    let buf = rmp_serde::to_vec(&ack)?;
    handle.datagram_mut(|sender: &mut Sender| {
        if let Err(e) = sender.send_datagram(buf.into()) {
            bail!(e);
        }
        anyhow::Ok(())
    })??;
    Ok(())
}
