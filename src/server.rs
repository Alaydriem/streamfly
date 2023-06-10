use std::{default::Default, path::Path, sync::Arc, task::Poll};

use anyhow::{bail, Result};
use futures::{future::poll_fn, lock::Mutex, AsyncReadExt, AsyncWriteExt};
use log::{debug, error, info};
use s2n_quic::{
    connection::{Handle, StreamAcceptor},
    provider::datagram::default::{Endpoint, Receiver, Sender},
    stream::ReceiveStream,
};

use crate::{
    io::{read_packet, write_packet},
    msg::{OpenStreamMsg, SubscribeAckMsg, SubscribeMsg},
    Reader, Writer,
};

pub async fn serve(addr: &str, cert: &Path, key: &Path) -> Result<()> {
    let datagram_provider = Endpoint::builder()
        .with_send_capacity(200)?
        .with_recv_capacity(200)?
        .build()
        .unwrap();

    let mut s = s2n_quic::Server::builder()
        .with_tls((cert, key))?
        .with_io(addr)?
        .with_datagram(datagram_provider)?
        .start()
        .unwrap();

    let all_handles = Arc::new(Mutex::new(Vec::default()));

    while let Some(conn) = s.accept().await {
        let all_handles_cloned = all_handles.clone();
        let (handle, acceptor) = conn.split();
        tokio::spawn(process_conn(all_handles_cloned, handle, acceptor));
    }

    Ok(())
}

async fn process_conn(
    all_handles: Arc<Mutex<Vec<(String, Handle)>>>,
    handle: Handle,
    mut acceptor: StreamAcceptor,
) -> Result<()> {
    let remote_addr = handle.remote_addr()?.to_string();
    info!("connection ++: {}", remote_addr);

    tokio::spawn(recv_datagrams_loop(all_handles.clone(), handle.clone()));

    while let Ok(Some(stream)) = acceptor.accept_receive_stream().await {
        tokio::spawn(process_recv_stream(all_handles.clone(), stream));
    }

    info!("connection --: {}", remote_addr);

    Ok(())
}

async fn recv_datagrams_loop(
    all_handles: Arc<Mutex<Vec<(String, Handle)>>>,
    handle: Handle,
) -> Result<()> {
    loop {
        match poll_fn(|cx| {
            match handle.datagram_mut(|recv: &mut Receiver| recv.poll_recv_datagram(cx)) {
                Ok(value) => value.map(Ok),
                Err(err) => Poll::Ready(Err(err)),
            }
        })
        .await?
        {
            Ok(buf) => {
                let msg = serde_json::from_slice::<SubscribeMsg>(&buf.to_vec())?;
                let topic = msg.topic;

                let msg = SubscribeAckMsg {
                    ok: true,
                    ..Default::default()
                };
                let buf = serde_json::to_vec(&msg)?;
                handle.datagram_mut(|sender: &mut Sender| {
                    if let Err(e) = sender.send_datagram(buf.into()) {
                        bail!(e);
                    }
                    anyhow::Ok(())
                })??;

                let mut all_handles = all_handles.lock().await;
                all_handles.push((topic, handle.clone()));
            }
            Err(e) => {
                bail!(e);
            }
        };
    }
}

async fn process_recv_stream(
    all_handles: Arc<Mutex<Vec<(String, Handle)>>>,
    stream: ReceiveStream,
) -> Result<()> {
    let remote_addr = stream.connection().remote_addr()?.to_string();
    let stream_id = stream.id();
    info!("recv_stream ++: {}|{}", remote_addr, stream_id);

    let mut reader: Reader = Box::pin(stream);
    let msg = read_packet::<OpenStreamMsg>(&mut reader).await?;
    let mut all_handles = all_handles.lock().await;
    let mut tx_list = vec![];

    for (topic, handle) in all_handles.as_mut_slice() {
        if topic == &msg.topic {
            if let Err(e) = open_stream(handle, &msg, &mut tx_list).await {
                error!("{}", e);
            }
        }
    }

    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf).await {
            Ok(length) => {
                if length == 0 {
                    break;
                }

                debug!("recv {} data from {}|{}", length, remote_addr, stream_id);

                for tx in &tx_list {
                    tx.send(buf[..length].into()).await?;
                }
            }

            Err(e) => {
                debug!("{}", e);

                for tx in &tx_list {
                    tx.close();
                }
                break;
            }
        }
    }

    info!("recv_stream --: {}|{}", remote_addr, stream_id);

    Ok(())
}

async fn open_stream(
    handle: &mut Handle,
    msg: &OpenStreamMsg,
    tx_list: &mut Vec<async_channel::Sender<Vec<u8>>>,
) -> Result<()> {
    let stream = handle.open_send_stream().await?;
    let remote_addr = stream.connection().remote_addr()?.to_string();
    let stream_id = stream.id();
    info!("send_stream ++: {}|{}", remote_addr, stream_id);

    let mut w: Writer = Box::pin(stream);
    write_packet(&mut w, msg.to_owned()).await?;

    let (tx, rx) = async_channel::unbounded::<Vec<u8>>();
    tx_list.push(tx);

    tokio::spawn(async move {
        loop {
            if let Ok(buf) = rx.recv().await {
                debug!("send {} data to {}|{}", buf.len(), remote_addr, stream_id);
                if let Err(e) = w.write_all(&buf).await {
                    error!("{}", e);
                    break;
                }
            } else {
                break;
            }
        }
        info!("send_stream --: {}|{}", remote_addr, stream_id);
    });

    Ok(())
}
