use std::{ collections::HashMap, sync::Arc };

use anyhow::{ bail, Result };
use async_channel::{ unbounded, Receiver, Sender };
use bytes::Bytes;
use futures::{ future::BoxFuture, lock::Mutex, AsyncWriteExt };
use tokio::task::JoinHandle;
use tracing::{ error, info, debug };
use s2n_quic::{
    connection::Handle,
    provider::datagram::default::Endpoint,
    stream::ReceiveStream,
    Connection,
};

use crate::{
    io::{ read_packet, recv_request, write_packet },
    msg::{ MsgOpenStream, MsgSubscribeStream, MsgType },
    stream::{ new_reader, new_writer },
    Reader,
    Writer,
    certificate::MtlsProvider,
};

pub async fn serve(
    addr: &str,
    provider: MtlsProvider,
    mutator: fn(&mut Vec<u8>) -> BoxFuture<'static, Result<Bytes, ()>>
) -> Result<JoinHandle<()>> {
    match
        s2n_quic::Server
            ::builder()
            .with_event(s2n_quic::provider::event::tracing::Subscriber::default())?
            .with_tls(provider)?
            .with_io(addr)?
            .with_datagram(
                Endpoint::builder().with_send_capacity(1400)?.with_recv_capacity(1400)?.build()?
            )?
            .start()
    {
        Err(e) => { bail!("{}", e) }
        Ok(mut s) => {
            info!("server is listening at: {}", addr);
            let task = tokio::spawn(async move {
                let mutator = mutator.clone();
                let all_handles = Arc::new(Mutex::new(HashMap::new()));
                let all_txs = Arc::new(Mutex::new(HashMap::new()));

                while let Some(conn) = s.accept().await {
                    let all_handles_cloned = all_handles.clone();
                    let all_txs_cloned = all_txs.clone();
                    tokio::spawn(async move {
                        if
                            let Err(e) = process_conn(
                                all_handles_cloned,
                                all_txs_cloned,
                                conn,
                                mutator
                            ).await
                        {
                            error!("process_conn: {}", e);
                        }
                    });
                }
                info!("server is closed");
            });
            return Ok(task);
        }
    }
}

async fn process_conn(
    all_handles: Arc<Mutex<HashMap<String, (String, Handle)>>>,
    all_txs: Arc<Mutex<HashMap<String, (String, Vec<Sender<Vec<u8>>>)>>>,
    conn: Connection,
    mutator: fn(&mut Vec<u8>) -> BoxFuture<'static, Result<Bytes, ()>>
) -> Result<()> {
    let (handle, mut acceptor) = conn.split();
    let remote_addr = handle.remote_addr()?.to_string();
    info!("connection ++: {}", remote_addr);

    let all_handles_cloned = all_handles.clone();
    let all_txs_cloned = all_txs.clone();
    tokio::spawn(async move {
        if let Err(e) = recv_datagrams_loop(all_handles_cloned, all_txs_cloned, handle).await {
            error!("recv_datagrams_loop: {}", e);
        }
    });

    while let Ok(Some(stream)) = acceptor.accept_receive_stream().await {
        let all_handles_cloned = all_handles.clone();
        let all_txs_cloned = all_txs.clone();
        tokio::spawn(async move {
            if
                let Err(e) = process_recv_stream(
                    all_handles_cloned,
                    all_txs_cloned,
                    stream,
                    mutator
                ).await
            {
                error!("process_recv_stream: {}", e);
            }
        });
    }

    info!("connection --: {}", remote_addr);
    if let Some((channel, _)) = all_handles.lock().await.remove(&remote_addr) {
        info!("subscriber --: [{}], {}", channel, remote_addr);
    }

    Ok(())
}

async fn recv_datagrams_loop(
    all_handles: Arc<Mutex<HashMap<String, (String, Handle)>>>,
    all_txs: Arc<Mutex<HashMap<String, (String, Vec<Sender<Vec<u8>>>)>>>,
    mut handle: Handle
) -> Result<()> {
    let remote_addr = handle.remote_addr()?.to_string();
    loop {
        let req = recv_request(&handle).await?;

        match req.msg_type {
            MsgType::Subcribe => {
                let msg: MsgSubscribeStream = rmp_serde::from_slice(&req.payload)?;
                info!("subscriber ++: [{}], {}", msg.channel, remote_addr);
                all_handles
                    .lock().await
                    .insert(remote_addr.to_owned(), (msg.channel.to_owned(), handle.to_owned()));

                for (stream_id, (channel, txs)) in all_txs.lock().await.iter_mut() {
                    if channel == &msg.channel {
                        match open_stream(&mut handle, &msg.channel, stream_id).await {
                            Ok(tx) => {
                                txs.push(tx);
                            }
                            Err(e) => {
                                error!("open_stream: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn process_recv_stream(
    all_handles: Arc<Mutex<HashMap<String, (String, Handle)>>>,
    all_txs: Arc<Mutex<HashMap<String, (String, Vec<Sender<Vec<u8>>>)>>>,
    stream: ReceiveStream,
    mutator: fn(&mut Vec<u8>) -> BoxFuture<'static, Result<Bytes, ()>>
) -> Result<()> {
    let remote_addr = stream.connection().remote_addr()?.to_string();
    let mut reader: Reader = new_reader(stream);
    let msg: MsgOpenStream = read_packet(&mut reader).await?;
    let stream_id = msg.stream_id.to_owned();
    info!("recv_stream ++: {}, {}", stream_id, remote_addr);

    let mut txs = Vec::new();
    for (channel, handle) in all_handles.lock().await.values_mut() {
        if channel == &msg.channel {
            match open_stream(handle, &msg.channel, &msg.stream_id).await {
                Ok(tx) => {
                    txs.push(tx);
                }
                Err(e) => {
                    error!("open_stream: {}", e);
                }
            }
        }
    }
    all_txs.lock().await.insert(msg.stream_id, (msg.channel, txs));

    tokio::spawn(async move {
        if
            let Err(e) = pipe_from_sender(
                reader,
                all_txs.to_owned(),
                &stream_id,
                &remote_addr,
                mutator
            ).await
        {
            error!("pipe_from_sender [{}], [{}]: {}", stream_id, remote_addr, e);
        }
        info!("recv_stream --: {}, {}", stream_id, remote_addr);
        all_txs.lock().await.remove(&stream_id);
    });

    Ok(())
}

async fn open_stream(
    handle: &mut Handle,
    channel: &str,
    stream_id: &str
) -> Result<Sender<Vec<u8>>> {
    let stream = handle.open_send_stream().await?;
    let remote_addr = stream.connection().remote_addr()?.to_string();
    let mut writer = new_writer(stream);
    let msg = MsgOpenStream {
        channel: channel.to_owned(),
        stream_id: stream_id.to_owned(),
    };
    write_packet(&mut writer, &msg).await?;
    info!("send_stream ++: {}, {}", msg.stream_id, remote_addr);

    let (tx, rx) = unbounded::<Vec<u8>>();
    tokio::spawn(async move {
        if let Err(e) = pipe_to_receiver(rx, writer, &msg.stream_id, &remote_addr).await {
            error!("pipe_to_receiver [{}], [{}]: {}", msg.stream_id, remote_addr, e);
        }
        info!("send_stream --: {}, {}", msg.stream_id, remote_addr);
    });

    Ok(tx)
}

async fn pipe_to_receiver(
    rx: Receiver<Vec<u8>>,
    mut writer: Writer,
    stream_id: &str,
    remote_addr: &str
) -> anyhow::Result<()> {
    while let Ok(buf) = rx.recv().await {
        debug!("send {} bytes: {}, {}", buf.len(), stream_id, remote_addr);
        writer.write_all(&buf).await?;
    }
    Ok(())
}

async fn pipe_from_sender(
    mut reader: Reader,
    all_txs: Arc<Mutex<HashMap<String, (String, Vec<Sender<Vec<u8>>>)>>>,
    stream_id: &str,
    remote_addr: &str,
    mutator: fn(&mut Vec<u8>) -> BoxFuture<'static, Result<Bytes, ()>>
) -> anyhow::Result<()> {
    let mut packet = Vec::<u8>::new();
    while let Some(mut buf) = reader.receive().await? {
        packet.append(&mut buf.to_vec());
        debug!("recv {} bytes: {}, {}", buf.len(), stream_id, remote_addr);

        buf = match mutator(&mut packet).await {
            Ok(buf) => buf,
            Err(_) => {
                continue;
            }
        };
        if let Some((_, txs)) = all_txs.lock().await.get_mut(stream_id) {
            for tx in txs.into_iter() {
                if tx.send(buf.to_owned().into()).await.is_err() {
                    tx.close();
                }
            }
            txs.retain(|tx| !tx.is_closed());
        }
    }
    Ok(())
}
