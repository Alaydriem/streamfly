use std::{path::Path, sync::Arc, task::Poll};

use anyhow::Result;
use futures::{future::poll_fn, lock::Mutex, AsyncReadExt, AsyncWriteExt};
use log::{debug, info};
use s2n_quic::{
    connection::{Handle, StreamAcceptor},
    provider::datagram::default::{Endpoint, Receiver},
    stream::ReceiveStream,
};

use crate::{
    io::{read_packet, write_packet},
    msg::{OpenStreamMsg, SubscribeMsg},
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
    info!("connection ++ : {}", remote_addr);

    loop {
        let all_handles_1 = all_handles.clone();
        let all_handles_2 = all_handles.clone();
        let handle_1 = handle.clone();

        tokio::select! {
            res = poll_fn(|cx| {
                match handle.datagram_mut(|recv: &mut Receiver| recv.poll_recv_datagram(cx)) {
                    Ok(value) => value.map(Ok),
                    Err(err) => Poll::Ready(Err(err)),
                }
            }) => {
                match res? {
                    Ok(buf) => {
                        let msg = serde_json::from_slice::<SubscribeMsg>(&buf.to_vec())?;
                        let mut all_handles = all_handles_1.lock().await;
                        all_handles.push((msg.topic, handle_1));
                    }
                    Err(e) => {
                        debug!("{}", e);
                        break ()
                    }
                };
            }


            res = acceptor.accept_receive_stream() => {
                if let Some(stream)= res? {
                    tokio::spawn(process_stream(all_handles_2, stream));
                } else {
                    break ()
                }
            }
        };
    }

    info!("connection -- : {}", remote_addr);

    Ok(())
}

async fn process_stream(
    all_handles: Arc<Mutex<Vec<(String, Handle)>>>,
    stream: ReceiveStream,
) -> Result<()> {
    let remote_addr = stream.connection().remote_addr()?.to_string();
    let stream_id = stream.id();
    info!("stream ++: {}|{}", remote_addr, stream_id);

    let mut reader: Reader = Box::pin(stream);
    let msg = read_packet::<OpenStreamMsg>(&mut reader).await?;
    let mut all_handles = all_handles.lock().await;
    let mut tx_list = vec![];

    for (topic, handle) in all_handles.as_mut_slice() {
        if topic == &msg.topic {
            let mut w: Writer = Box::pin(handle.open_send_stream().await?);
            write_packet(&mut w, msg.to_owned()).await?;

            let (tx, rx) = async_channel::unbounded::<Vec<u8>>();
            tx_list.push(tx);

            tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(buf) => {
                            w.write_all(&buf).await?;
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                w.close().await?;
                anyhow::Ok(())
            });
        }
    }

    let mut buf = [0u8; 1024];
    loop {
        let length = reader.read(&mut buf).await?;
        if length == 0 {
            debug!("read length == 0");
            for tx in &tx_list {
                tx.close();
            }
            break;
        }

        for tx in &tx_list {
            tx.send(buf[..length].into()).await?;
        }
    }

    info!("stream --: {}|{}", remote_addr, stream_id);

    Ok(())
}
